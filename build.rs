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

    let background_layer = &map.layers[0];
    let background_tiles = extract_tiles(&background_layer.tiles);

    let foreground_layer = &map.layers[1];
    let foreground_tiles = extract_tiles(&foreground_layer.tiles);

    let output = quote! {
        pub const BACKGROUND_MAP: &[u16] = &[#(#background_tiles),*];
        pub const FOREGROUND_MAP: &[u16] = &[#(#foreground_tiles),*];
        pub const WIDTH: u32 = #width;
        pub const HEIGHT: u32 = #height;
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
