//! Built-in tile renderers for demo and testing.

/// Elevation-style gradient: blue (low) → green → yellow → red → white (high).
pub fn elevation(z: u32, x: u32, y: u32) -> Vec<u8> {
    let mut data = vec![0u8; 256 * 256 * 4];
    for py in 0usize..256 {
        for px in 0usize..256 {
            let idx = (py * 256 + px) * 4;
            let elev = ((x as f64 * 256.0 + px as f64) / (z.max(1) as f64 * 256.0)
                + (y as f64 * 256.0 + py as f64) / (z.max(1) as f64 * 256.0))
                % 1.0;
            let (r, g, b) = if elev < 0.25 {
                (0, (elev * 4.0 * 255.0) as u8, 180)
            } else if elev < 0.5 {
                let t = (elev - 0.25) * 4.0;
                (
                    (t * 120.0) as u8,
                    (180.0 + t * 75.0) as u8,
                    ((1.0 - t) * 180.0) as u8,
                )
            } else if elev < 0.75 {
                let t = (elev - 0.5) * 4.0;
                ((120.0 + t * 135.0) as u8, ((1.0 - t) * 255.0) as u8, 0)
            } else {
                let t = (elev - 0.75) * 4.0;
                let v = ((1.0 - t) * 255.0) as u8;
                (v, v, v)
            };
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }
    data
}

/// Land cover style: colored blocks based on tile coordinates.
pub fn landcover(_z: u32, x: u32, y: u32) -> Vec<u8> {
    let colors: [(u8, u8, u8); 8] = [
        (34, 139, 34),   // forest green
        (154, 205, 50),  // grassland
        (65, 105, 225),  // water blue
        (210, 180, 140), // bare tan
        (169, 169, 169), // built-up gray
        (255, 215, 0),   // cropland gold
        (0, 100, 0),     // wetland dark green
        (255, 69, 0),    // emphasis red
    ];
    let mut data = vec![0u8; 256 * 256 * 4];
    for py in 0..256 {
        for px in 0..256 {
            let idx = (py as usize * 256 + px as usize) * 4;
            let ci =
                ((x.wrapping_mul(7) ^ y.wrapping_mul(13) ^ (px / 64) ^ (py / 64)) % 8) as usize;
            let (r, g, b) = colors[ci];
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }
    data
}

/// Generate a checkerboard test pattern.
pub fn checkerboard(_z: u32, _x: u32, _y: u32) -> Vec<u8> {
    let mut data = vec![0u8; 256 * 256 * 4];
    for py in 0..256 {
        for px in 0..256 {
            let idx = (py as usize * 256 + px as usize) * 4;
            let check = ((px / 32) + (py / 32)) % 2 == 0;
            if check {
                data[idx] = 200;
                data[idx + 1] = 200;
                data[idx + 2] = 200;
            } else {
                data[idx] = 240;
                data[idx + 1] = 240;
                data[idx + 2] = 240;
            }
            data[idx + 3] = 255;
        }
    }
    data
}
