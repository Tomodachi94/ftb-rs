// Copyright © 2014, Peter Atashian

#![feature(core, env, io, path, plugin, std_misc)]
#![plugin(regex_macros)]

extern crate image;
extern crate lodepng;
extern crate regex;
extern crate "rustc-serialize" as serialize;

use image::{GenericImage, ImageBuffer, Pixel, Rgba, RgbaImage};
use image::ColorType::RGBA;
use std::borrow::ToOwned;
use std::collections::HashMap;
use std::old_io::{BufferedReader, BufferedWriter, File};
use std::old_io::fs::{PathExtensions, copy, walk_dir};
use std::num::{Float};

mod tilesheets;
#[allow(unused_variables, non_snake_case)]
mod oregen;

type FloatImage = ImageBuffer<Rgba<f32>, Vec<f32>>;
fn save(img: &RgbaImage, path: &Path) {
    image::save_buffer(path, img.as_slice(), img.width(), img.height(), RGBA(8)).unwrap();
}

fn read_gt_lang() -> HashMap<String, String> {
    let path = Path::new(r"work/GregTech.lang");
    let mut file = File::open(&path).unwrap();
    let data = file.read_to_string().unwrap();
    let reg = regex!(r"S:([\w\.]+?)=(.+?)\r?\n");
    reg.captures_iter(data.as_slice()).map(|cap|
        (cap.at(1).unwrap().to_owned(), cap.at(2).unwrap().to_owned())
    ).collect()
}
trait Srgb {
    type Linear;
    fn decode(&self) -> <Self as Srgb>::Linear;
}
impl Srgb for Rgba<u8> {
    type Linear = Rgba<f32>;
    fn decode(&self) -> Rgba<f32> {
        fn dec(x: u8) -> f32 {
            let x = x as f32 * (1. / 255.);
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / (1. + 0.055)).powf(2.4)
            }
        }
        let p = Rgba([dec(self[0]), dec(self[1]), dec(self[2]), dec(self[3])]);
        Rgba([p[0] * p[3], p[1] * p[3], p[2] * p[3], p[3]])
    }
}
fn decode_srgb(img: &RgbaImage) -> FloatImage {
    let (w, h) = img.dimensions();
    ImageBuffer::from_fn(w, h, |x, y| img[(x, y)].decode())
}
trait Linear {
    type Srgb;
    fn encode(&self) -> <Self as Linear>::Srgb;
}
impl Linear for Rgba<f32> {
    type Srgb = Rgba<u8>;
    fn encode(&self) -> Rgba<u8> {
        fn enc(x: f32) -> u8 {
            let x = if x <= 0.0031308 {
                x * 12.92
            } else {
                x.powf(1. / 2.4) * (1. + 0.055) - 0.055
            };
            (x * 255.).round().max(0.).min(255.) as u8
        }
        let p = if self[3] > 0.0001 {
            Rgba([self[0] / self[3], self[1] / self[3], self[2] / self[3], self[3]])
        } else {
            Rgba([0., 0., 0., 0.])
        };
        Rgba([enc(p[0]), enc(p[1]), enc(p[2]), enc(p[3])])
    }
}
fn encode_srgb(img: &FloatImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    ImageBuffer::from_fn(w, h, |x, y| img[(x, y)].encode())
}
fn resize(img: &FloatImage, width: u32, height: u32) -> FloatImage {
    let (w, h) = img.dimensions();
    assert!(width.cmp(&w) == height.cmp(&h));
    if width < w {
        let (rw, rh) = (w as f32 / (width as f32), h as f32 / (height as f32));
        ImageBuffer::from_fn(width, height, |x: u32, y: u32| {
            let (x1, x2) = ((x as f32 * rw) as u32, ((x + 1) as f32 * rw) as u32);
            let (y1, y2) = ((y as f32 * rh) as u32, ((y + 1) as f32 * rh) as u32);
            let (mut r, mut g, mut b, mut a) = (0., 0., 0., 0.);
            for xx in range(x1, x2) {
                for yy in range(y1, y2) {
                    let p = img[(xx, yy)];
                    r += p[0];
                    g += p[1];
                    b += p[2];
                    a += p[3];
                }
            }
            let m = 1. / (((x2 - x1) * (y2 - y1)) as f32);
            Rgba([r * m, g * m, b * m, a * m])
        })
    } else if width == w {
        img.clone()
    } else {
        let (rw, rh) = (w as f32 / (width as f32), h as f32 / (height as f32));
        ImageBuffer::from_fn(width, height, |x: u32, y: u32| {
            let xx = (x as f32 * rw) as u32;
            let yy = (y as f32 * rh) as u32;
            img[(xx, yy)]
        })
    }
}
fn grab_crops() {
    let path = Path::new(r"C:\Users\Peter\Minecraft\Wiki\GT Dev\assets\ic2\textures\blocks\crop");
    let out = Path::new(r"work\tilesheets\Crops");
    let reg = regex!(r"blockCrop\.(.*)\.(.*)");
    for path in walk_dir(&path).unwrap() {
        if !path.is_file() { continue }
        if path.extension_str() != Some("png") { continue }
        let name = path.filestem_str().unwrap();
        let cap = reg.captures(name).unwrap();
        let new = format!("Crop {} ({}).png", cap.at(1).unwrap(), cap.at(2).unwrap());
        let newp = out.join(new);
        copy(&path, &newp).unwrap();
    }
}
fn check_lang_dups() {
    let lang = read_gt_lang();
    let mut stuff = HashMap::new();
    for (key, val) in lang.iter() {
        if !key.as_slice().contains(".name") { continue }
        if key.as_slice().contains(".tooltip") { continue }
        if key.as_slice().contains("gt.recipe") { continue }
        if key.as_slice().contains("DESCRIPTION") { continue }
        match stuff.get(val) {
            Some(other) => {
                println!("Collision for {}", val);
                println!("{} && {}", key, other);
            },
            None => (),
        }
        stuff.insert(val.clone(), key);
    }
}
fn import_old_tilesheet(name: &str) {
    let path = Path::new(r"work/tilesheets/import.txt");
    if !path.is_file() { return }
    println!("Importing old tilesheet");
    let mut file = File::open(&path).unwrap();
    let data = file.read_to_string().unwrap();
    let name = format!("work/tilesheets/Tilesheet {}.txt", name);
    let path = Path::new(&name[]);
    let mut out = File::create(&path).unwrap();
    let reg = regex!(r"Edit\s+[0-9]+\s+(.+?)\s+[A-Z0-9]+\s+([0-9]+)\s+([0-9]+)\s+16px, 32px\r?\n");
    for cap in reg.captures_iter(data.as_slice()) {
        let name = cap.at(1).unwrap();
        let x = cap.at(2).unwrap();
        let y = cap.at(3).unwrap();
        (writeln!(&mut out, "{} {} {}", x, y, name)).unwrap();
    }
}
fn fix_lang() {
    let path = Path::new(r"work/GregTech.lang");
    let mut file = File::open(&path).unwrap();
    let data = file.read_to_string().unwrap();
    let data = regex!("\r").replace_all(&data[], "");
    let data = regex!("(blockores\\.[0-9]{1,3}\\.name=.*)").replace_all(&data[], "$1 (Stone)");
    let data = regex!("(blockores\\.1[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Netherrack)");
    let data = regex!("(blockores\\.2[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Endstone)");
    let data = regex!("(blockores\\.3[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Black Granite)");
    let data = regex!("(blockores\\.4[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Red Granite)");
    let data = regex!("(blockores\\.16[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Stone)");
    let data = regex!("(blockores\\.17[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Netherrack)");
    let data = regex!("(blockores\\.18[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Endstone)");
    let data = regex!("(blockores\\.19[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Black Granite)");
    let data = regex!("(blockores\\.20[0-9]{3}\\.name=.*)").replace_all(&data[], "$1 (Red Granite)");
    let data = regex!("(S:fluid\\..*=.*)").replace_all(&data[], "$1 (Fluid)");
    drop(file);
    let mut file = File::create(&path).unwrap();
    file.write_str(&data[]).unwrap();
}

fn dump_oredict() {
    let lang = read_gt_lang();
    let reg = regex!("^([0-9]+)x(.+)@([0-9]+)$");
    let fin = File::open(&Path::new(r"work/neiintegration_oredict.csv")).unwrap();
    let fout = File::create(&Path::new(r"work/oredict.txt")).unwrap();
    let mut fin = BufferedReader::new(fin);
    let mut fout = BufferedWriter::new(fout);
    for line in fin.lines() {
        let line = line.unwrap();
        let parts = line.trim().split(',').collect::<Vec<_>>();
        assert!(parts.len() == 5);
        let tag = parts[0];
        let stack = parts[1];
        let _id = parts[2];
        let wildcard = parts[3];
        let modname = parts[4];
        if modname != "gregtech" { continue }
        assert!(wildcard == "false");
        let cap = reg.captures(stack).unwrap();
        let quantity = cap.at(1).unwrap();
        assert!(quantity == "1");
        let item = cap.at(2).unwrap();
        let meta = cap.at(3).unwrap();
        let unlocal = format!("{}.{}.name", item, meta);
        if !lang.contains_key(&unlocal) {
            println!("Missing: {}", unlocal);
            continue
        }
        writeln!(&mut fout, "{}!{}!GT!!", tag, lang[unlocal]).unwrap();
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let args: Vec<_> = args.iter().map(|x| &**x).collect();
    match &args[1..] {
        ["update", name] => tilesheets::update_tilesheet(name, &[16, 32], false),
        ["overwrite", name] => tilesheets::update_tilesheet(name, &[16, 32], true),
        ["import", name] => import_old_tilesheet(name),
        ["fixlang"] => fix_lang(),
        ["langdup"] => check_lang_dups(),
        ["dumporedict"] => dump_oredict(),
        ["oregen"] => oregen::oregen(),
        ["crops"] => grab_crops(),
        _ => println!("Invalid arguments"),
    }
}
