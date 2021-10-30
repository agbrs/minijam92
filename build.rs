use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use quote::quote;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable must be specified");

    let playground_filename = "map/playground.tmx";
    println!("cargo:rerun-if-changed={}", playground_filename);

    let map = tiled::parse_file(Path::new(playground_filename)).unwrap();

    let width = map.width;
    let height = map.height;

    let cloud_layer = &map.layers[0];
    let cloud_tiles = extract_tiles(&cloud_layer.tiles);

    let background_layer = &map.layers[1];
    let background_tiles = extract_tiles(&background_layer.tiles);

    let foreground_layer = &map.layers[2];
    let foreground_tiles = extract_tiles(&foreground_layer.tiles);

    let slime_spawns = map.object_groups[0]
        .objects
        .iter()
        .filter(|object| &object.obj_type == "Slime Spawn")
        .map(|object| (object.x as u16, object.y as u16))
        .collect::<Vec<_>>();
    let slimes_x = slime_spawns.iter().map(|pos| pos.0);
    let slimes_y = slime_spawns.iter().map(|pos| pos.1);

    let mut tile_types = HashMap::new();

    for tile in map.tilesets[0].tiles.iter() {
        if let Some("Collision") = tile.tile_type.as_deref() {
            tile_types.insert(tile.id, 1u8);
        }
    }

    let tile_types =
        (0..map.tilesets[0].tilecount.unwrap()).map(|id| tile_types.get(&(id + 1)).unwrap_or(&0));

    let output = quote! {
        pub const CLOUD_MAP: &[u16] = &[#(#cloud_tiles),*];
        pub const BACKGROUND_MAP: &[u16] = &[#(#background_tiles),*];
        pub const FOREGROUND_MAP: &[u16] = &[#(#foreground_tiles),*];
        pub const WIDTH: u32 = #width;
        pub const HEIGHT: u32 = #height;

        pub const SLIME_SPAWNS_X: &[u16] = &[#(#slimes_x),*];
        pub const SLIME_SPAWNS_Y: &[u16] = &[#(#slimes_y),*];

        pub const TILE_TYPES: &[u8] = &[#(#tile_types),*];
    };

    let output_file = File::create(format!("{}/tilemap.rs", out_dir))
        .expect("failed to open tilemap.rs file for writing");
    let mut writer = BufWriter::new(output_file);

    write!(&mut writer, "{}", output).unwrap();
}

fn extract_tiles<'a>(layer: &'a tiled::LayerData) -> impl Iterator<Item = u16> + 'a {
    match layer {
        tiled::LayerData::Finite(tiles) => {
            tiles.iter().flat_map(|row| row.iter().map(|tile| tile.gid))
        }
        _ => unimplemented!("cannot use infinite layer"),
    }
    .map(|tileid| get_map_id(tileid))
}

fn get_map_id(tileid: u32) -> u16 {
    match tileid {
        0 => 0,
        i => i as u16 - 1,
    }
}
