use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;

use crate::world::palette::PaletteBlockKind;

pub struct ChunkGenerationParams<'lvl> {
    pub cx: i32,
    pub cz: i32,
    pub random: &'lvl mut StdRng,
    pub noise: &'lvl mut Perlin,
}

fn fbm2d(perlin: &Perlin, x: f64, z: f64, octaves: u32) -> f64 {
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut sum = 0.0;
    let mut norm = 0.0;

    for _ in 0..octaves {
        sum += amp * perlin.get([x * freq, z * freq]);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }

    sum / norm
}

pub fn do_chunk_generation<F: FnMut(i32, i32, i32, PaletteBlockKind)>(
    params: &mut ChunkGenerationParams,
    mut place_tile: F,
) {
    let noise = &mut *params.noise;

    let world_scale = 0.0007;
    let base_y = 20.0;
    let amp_y = 35.0;
    let octaves = 5;

    for lx in 0..16 {
        for lz in 0..16 {
            let wx = f64::from(lx + params.cx * 16);
            let wz = f64::from(lz + params.cz * 16);

            let n = fbm2d(noise, wx * world_scale, wz * world_scale, octaves);
            let mut h = base_y + (n * amp_y);
            h = h.clamp(0.0, 300.0);

            let y = h.round_ties_even() as i32;

            let kind = if y >= 3 {
                PaletteBlockKind::Stone
            } else {
                PaletteBlockKind::Bedrock
            };

            place_tile(lx, y, lz, kind);
            place_tile(lx, y - 1, lz, kind);
        }
    }
}
