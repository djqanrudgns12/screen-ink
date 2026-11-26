//! Snipaste 대조 판독(08 §5.4): F1 전·후 화면 PNG(같은 모니터, 같은 크기)에서 Snipaste 강조 사각형을 찾는다.
//!
//!   snipaste-diff <전.png> <후.png> <커서x> <커서y> [--inset N]
//!   → 표준 출력 한 줄: `x,y,w,h`(이미지 좌표) 또는 `none`
//!
//! 판독은 순수 함수 `ink_vision::diff_rect::detect_snipaste_highlight`(단위 시험 있음). `--inset N`은 Snipaste 테두리가
//! 사각형 안쪽에 그려져 결과가 N px 작게 나오는 것을 되돌린다(알려진 창으로 한 번 재서 정함, compare-snipaste.ps1 -Calibrate).

use std::path::Path;

fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(file));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; r.output_buffer_size().ok_or("png too large")?];
    let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let px = &buf[..info.buffer_size()];
    let mut bgra = Vec::with_capacity((info.width * info.height * 4) as usize);
    match info.color_type {
        png::ColorType::Rgb => px.chunks_exact(3).for_each(|c| bgra.extend_from_slice(&[c[2], c[1], c[0], 255])),
        png::ColorType::Rgba => px.chunks_exact(4).for_each(|c| bgra.extend_from_slice(&[c[2], c[1], c[0], 255])),
        png::ColorType::Grayscale => px.iter().for_each(|&g| bgra.extend_from_slice(&[g, g, g, 255])),
        png::ColorType::GrayscaleAlpha => px.chunks_exact(2).for_each(|c| bgra.extend_from_slice(&[c[0], c[0], c[0], 255])),
        png::ColorType::Indexed => return Err("indexed png not expanded".into()),
    }
    Ok((info.width, info.height, bgra))
}

fn run() -> Result<String, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        return Err("쓰는 법: snipaste-diff <전.png> <후.png> <커서x> <커서y> [--inset N]".into());
    }
    let inset: i32 = args.iter().position(|a| a == "--inset").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0);
    let (bw, bh, before) = read_png(Path::new(&args[0]))?;
    let (aw, ah, after) = read_png(Path::new(&args[1]))?;
    if (bw, bh) != (aw, ah) {
        return Err(format!("크기가 달라요: {bw}x{bh} / {aw}x{ah}"));
    }
    let x: i32 = args[2].parse().map_err(|_| "커서 x가 숫자가 아니에요")?;
    let y: i32 = args[3].parse().map_err(|_| "커서 y가 숫자가 아니에요")?;
    Ok(match ink_vision::diff_rect::detect_snipaste_highlight(&before, &after, bw, bh, Some((x, y))) {
        Some(r) => format!("{},{},{},{}", r.x - inset, r.y - inset, r.w + 2 * inset, r.h + 2 * inset),
        None => "none".into(),
    })
}

fn main() {
    match run() {
        Ok(line) => println!("{line}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
