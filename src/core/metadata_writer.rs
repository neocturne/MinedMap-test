//! The [MetadataWriter] and related types

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{core::common::*, io::fs, world::de};

/// Minimum and maximum X and Z tile coordinates for a mipmap level
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Bounds {
	/// Minimum X coordinate
	min_x: i32,
	/// Maximum X coordinate
	max_x: i32,
	/// Minimum Z coordinate
	min_z: i32,
	/// Maximum Z coordinate
	max_z: i32,
}

/// Mipmap level information in viewer metadata file
#[derive(Debug, Serialize)]
struct Mipmap<'t> {
	/// Minimum and maximum tile coordinates of the mipmap level
	bounds: Bounds,
	/// Map of populated tiles for the mipmap level
	regions: &'t TileCoordMap,
}

/// Initial spawn point for new players
#[derive(Debug, Serialize)]
struct Spawn {
	/// Spawn X coordinate
	x: i32,
	/// Spawn Z coordinate
	z: i32,
}

/// Viewer metadata JSON data structure
#[derive(Debug, Serialize)]
struct Metadata<'t> {
	/// Tile information for each mipmap level
	mipmaps: Vec<Mipmap<'t>>,
	/// Initial spawn point for new players
	spawn: Spawn,
}

/// The MetadataWriter is used to generate the viewer metadata file
pub struct MetadataWriter<'a> {
	/// Common MinedMap configuration from command line
	config: &'a Config,
	/// Map of generated tiles for each mipmap level
	tiles: &'a [TileCoordMap],
}

impl<'a> MetadataWriter<'a> {
	/// Creates a new MetadataWriter
	pub fn new(config: &'a Config, tiles: &'a [TileCoordMap]) -> Self {
		MetadataWriter { config, tiles }
	}

	/// Helper to construct a [Mipmap] data structure from a [TileCoordMap]
	fn mipmap_entry(regions: &TileCoordMap) -> Mipmap {
		let mut min_x = i32::MAX;
		let mut max_x = i32::MIN;
		let mut min_z = i32::MAX;
		let mut max_z = i32::MIN;

		for (&z, xs) in &regions.0 {
			if z < min_z {
				min_z = z;
			}
			if z > max_z {
				max_z = z;
			}

			for &x in xs {
				if x < min_x {
					min_x = x;
				}
				if x > max_x {
					max_x = x;
				}
			}
		}

		Mipmap {
			bounds: Bounds {
				min_x,
				max_x,
				min_z,
				max_z,
			},
			regions,
		}
	}

	/// Reads and deserializes the `level.dat` of the Minecraft save data
	fn read_level_dat(&self) -> Result<de::LevelDat> {
		crate::nbt::data::from_file(&self.config.level_dat_path).context("Failed to read level.dat")
	}

	/// Generates [Spawn] data from a [de::LevelDat]
	fn spawn(level_dat: &de::LevelDat) -> Spawn {
		Spawn {
			x: level_dat.data.spawn_x,
			z: level_dat.data.spawn_z,
		}
	}

	/// Runs the viewer metadata file generation
	pub fn run(self) -> Result<()> {
		let level_dat = self.read_level_dat()?;

		let mut metadata = Metadata {
			mipmaps: Vec::new(),
			spawn: Self::spawn(&level_dat),
		};

		for tile_map in self.tiles.iter() {
			metadata.mipmaps.push(Self::mipmap_entry(tile_map));
		}

		fs::create_with_tmpfile(&self.config.metadata_path, |file| {
			serde_json::to_writer(file, &metadata).context("Failed to write metadata")
		})
	}
}
