use std::{env, fs, path::Path, path::PathBuf};

include!("src/glyph.rs");

/// Small-icon sizes for 100% to 400% scale, so `LoadIconMetric` never has to stretch a bitmap.
const SIZES: [u32; 8] = [16, 20, 24, 28, 32, 40, 48, 64];

const VERSION_INFO: &str = r#"
1 VERSIONINFO
FILEVERSION 0,1,0,0
PRODUCTVERSION 0,1,0,0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "Haik Asatryan"
      VALUE "FileDescription", "Wakeshot tray spike"
      VALUE "FileVersion", "0.1.0"
      VALUE "OriginalFilename", "tray-spike.exe"
      VALUE "ProductName", "Wakeshot tray spike"
      VALUE "ProductVersion", "0.1.0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#;

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let mut rc = String::new();
    // Ids must match the ICON_* constants in src/main.rs.
    for (id, filled, rgb) in [
        (101, true, ON_DARK),
        (102, false, ON_DARK),
        (103, true, ON_LIGHT),
        (104, false, ON_LIGHT),
    ] {
        let path = out.join(format!("icon{id}.ico"));
        fs::write(&path, ico(filled, rgb)).unwrap();
        rc += &format!("{id} ICON \"{}\"\n", rc_path(&path));
    }
    let wav = out.join("chime.wav");
    fs::write(&wav, chime_wav()).unwrap();
    rc += &format!("CHIME WAVE \"{}\"\n", rc_path(&wav));
    rc += &format!(
        "1 24 \"{}\"\n",
        rc_path(&root.join("tray-spike.exe.manifest"))
    );
    rc += VERSION_INFO;

    let rc_file = out.join("app.rc");
    fs::write(&rc_file, rc).unwrap();
    embed_resource::compile(&rc_file, embed_resource::NONE)
        .manifest_required()
        .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/glyph.rs");
    println!("cargo:rerun-if-changed=tray-spike.exe.manifest");
}

fn rc_path(p: &Path) -> String {
    p.display().to_string().replace('\\', "\\\\")
}

fn ico(filled: bool, rgb: [u8; 3]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = SIZES
        .iter()
        .map(|&s| dib(s, &glyph_rgba(s, filled, rgb)))
        .collect();
    let mut out = Vec::new();
    out.extend(0u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend((SIZES.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * SIZES.len() as u32;
    for (&s, img) in SIZES.iter().zip(&images) {
        out.extend([s as u8, s as u8, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((img.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in images {
        out.extend(img);
    }
    out
}

/// One ICO image: BITMAPINFOHEADER, bottom-up BGRA, then an all-zero AND mask (alpha does the work).
fn dib(s: u32, rgba: &[u8]) -> Vec<u8> {
    let mask_row = s.div_ceil(32) * 4;
    let mut out = Vec::new();
    out.extend(40u32.to_le_bytes());
    out.extend((s as i32).to_le_bytes());
    out.extend((2 * s as i32).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend((s * s * 4 + mask_row * s).to_le_bytes());
    out.extend([0u8; 16]);
    for y in (0..s).rev() {
        for x in 0..s {
            let i = ((y * s + x) * 4) as usize;
            out.extend([rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    out.extend(vec![0u8; (mask_row * s) as usize]);
    out
}

/// Two short tones, 16-bit mono PCM, as a stand-in for a bundled sound file.
fn chime_wav() -> Vec<u8> {
    const RATE: u32 = 22_050;
    let mut samples: Vec<i16> = Vec::new();
    for (freq, secs) in [(880.0f32, 0.12f32), (1318.5, 0.18)] {
        let n = (RATE as f32 * secs) as u32;
        for i in 0..n {
            let t = i as f32 / RATE as f32;
            let env = (-t * 18.0).exp() * (1.0 - (-(t * 400.0)).exp());
            let v = (2.0 * std::f32::consts::PI * freq * t).sin() * env * 0.35;
            samples.push((v * i16::MAX as f32) as i16);
        }
    }
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend(b"RIFF");
    out.extend((36 + data_len).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(RATE.to_le_bytes());
    out.extend((RATE * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(data_len.to_le_bytes());
    for s in samples {
        out.extend(s.to_le_bytes());
    }
    out
}
