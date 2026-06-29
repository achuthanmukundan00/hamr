//! Armin says hi! A fun easter egg with animated XBM art.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/armin.ts`.

use crate::modes::interactive::components::tui_shim::Component;
use crate::modes::interactive::theme::theme::theme;

// XBM image: 31x36 pixels, LSB first, 1=background, 0=foreground
const WIDTH: usize = 31;
const HEIGHT: usize = 36;
const BITS: [u8; 144] = [
    0xff, 0xff, 0xff, 0x7f, 0xff, 0xf0, 0xff, 0x7f, 0xff, 0xed, 0xff, 0x7f, 0xff, 0xdb, 0xff, 0x7f,
    0xff, 0xb7, 0xff, 0x7f, 0xff, 0x77, 0xfe, 0x7f, 0x3f, 0xf8, 0xfe, 0x7f, 0xdf, 0xff, 0xfe, 0x7f,
    0xdf, 0x3f, 0xfc, 0x7f, 0x9f, 0xc3, 0xfb, 0x7f, 0x6f, 0xfc, 0xf4, 0x7f, 0xf7, 0x0f, 0xf7, 0x7f,
    0xf7, 0xff, 0xf7, 0x7f, 0xf7, 0xff, 0xe3, 0x7f, 0xf7, 0x07, 0xe8, 0x7f, 0xef, 0xf8, 0x67, 0x70,
    0x0f, 0xff, 0xbb, 0x6f, 0xf1, 0x00, 0xd0, 0x5b, 0xfd, 0x3f, 0xec, 0x53, 0xc1, 0xff, 0xef, 0x57,
    0x9f, 0xfd, 0xee, 0x5f, 0x9f, 0xfc, 0xae, 0x5f, 0x1f, 0x78, 0xac, 0x5f, 0x3f, 0x00, 0x50, 0x6c,
    0x7f, 0x00, 0xdc, 0x77, 0xff, 0xc0, 0x3f, 0x78, 0xff, 0x01, 0xf8, 0x7f, 0xff, 0x03, 0x9c, 0x78,
    0xff, 0x07, 0x8c, 0x7c, 0xff, 0x0f, 0xce, 0x78, 0xff, 0xff, 0xcf, 0x7f, 0xff, 0xff, 0xcf, 0x78,
    0xff, 0xff, 0xdf, 0x78, 0xff, 0xff, 0xdf, 0x7d, 0xff, 0xff, 0x3f, 0x7e, 0xff, 0xff, 0xff, 0x7f,
];

const BYTES_PER_ROW: usize = 4; // ceil(31/8) = 4
const DISPLAY_HEIGHT: usize = 18; // ceil(36/2) = 18

type Effect = EffectType;

#[derive(Clone, Copy, Debug, PartialEq)]
enum EffectType {
    Typewriter,
    Scanline,
    Rain,
    Fade,
    Crt,
    Glitch,
    Dissolve,
}

const EFFECTS: [EffectType; 7] = [
    EffectType::Typewriter,
    EffectType::Scanline,
    EffectType::Rain,
    EffectType::Fade,
    EffectType::Crt,
    EffectType::Glitch,
    EffectType::Dissolve,
];

/// Get pixel at (x, y): true = foreground, false = background
fn get_pixel(x: usize, y: usize) -> bool {
    if y >= HEIGHT {
        return false;
    }
    let byte_index = y * BYTES_PER_ROW + (x / 8);
    let bit_index = x % 8;
    ((BITS[byte_index] >> bit_index) & 1) == 0
}

/// Get the character for a cell (2 vertical pixels packed into a half-block)
fn get_char(x: usize, row: usize) -> &'static str {
    let upper = get_pixel(x, row * 2);
    let lower = get_pixel(x, row * 2 + 1);
    match (upper, lower) {
        (true, true) => "█",
        (true, false) => "▀",
        (false, true) => "▄",
        (false, false) => " ",
    }
}

/// Build the final image grid.
fn build_final_grid() -> Vec<Vec<String>> {
    let mut grid = Vec::with_capacity(DISPLAY_HEIGHT);
    for row in 0..DISPLAY_HEIGHT {
        let mut line = Vec::with_capacity(WIDTH);
        for x in 0..WIDTH {
            line.push(get_char(x, row).to_string());
        }
        grid.push(line);
    }
    grid
}

// ---------------------------------------------------------------------------
// Effect state types
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct RainDrop {
    y: i32,         // current falling position
    settled: usize, // how many rows from bottom are settled
}

/// The effect state (one variant per effect type).
enum EffectState {
    Typewriter {
        pos: usize,
    },
    Scanline {
        row: usize,
    },
    Rain {
        drops: Vec<RainDrop>,
    },
    Fade {
        positions: Vec<(usize, usize)>,
        idx: usize,
    },
    Crt {
        expansion: usize,
    },
    Glitch {
        phase: usize,
        glitch_frames: usize,
    },
    Dissolve {
        positions: Vec<(usize, usize)>,
        idx: usize,
    },
}

// ---------------------------------------------------------------------------
// ArminComponent
// ---------------------------------------------------------------------------

pub struct ArminComponent {
    effect: EffectType,
    final_grid: Vec<Vec<String>>,
    current_grid: Vec<Vec<String>>,
    effect_state: EffectState,
    cached_lines: Vec<String>,
    cached_width: u16,
    grid_version: usize,
    cached_version: usize,
    pub tick_count: usize,
}

impl ArminComponent {
    pub fn new(effect_index: Option<usize>) -> Self {
        let idx = effect_index.unwrap_or(0) % EFFECTS.len();
        let effect = EFFECTS[idx];
        let final_grid = build_final_grid();
        let current_grid = create_empty_grid();
        let effect_state = init_effect_state(&effect, &final_grid);

        Self {
            effect,
            final_grid,
            current_grid,
            effect_state,
            cached_lines: Vec::new(),
            cached_width: 0,
            grid_version: 0,
            cached_version: usize::MAX,
            tick_count: 0,
        }
    }

    /// Choose a random effect.
    pub fn new_random() -> Self {
        // Use a simple pseudo-random seed based on time
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let idx = (seed as usize) % EFFECTS.len();
        Self::new(Some(idx))
    }

    /// Advance animation state. Returns true when done.
    pub fn tick_effect(&mut self) -> bool {
        self.tick_count += 1;
        let done = match self.effect {
            EffectType::Typewriter => self.tick_typewriter(),
            EffectType::Scanline => self.tick_scanline(),
            EffectType::Rain => self.tick_rain(),
            EffectType::Fade => self.tick_fade(),
            EffectType::Crt => self.tick_crt(),
            EffectType::Glitch => self.tick_glitch(),
            EffectType::Dissolve => self.tick_dissolve(),
        };
        if !done {
            self.grid_version += 1;
        }
        done
    }

    // --- Effect ticks ---

    fn tick_typewriter(&mut self) -> bool {
        let pos = match &mut self.effect_state {
            EffectState::Typewriter { pos } => pos,
            _ => return true,
        };
        let pixels_per_frame = 3;

        for _ in 0..pixels_per_frame {
            let row = *pos / WIDTH;
            let x = *pos % WIDTH;
            if row >= DISPLAY_HEIGHT {
                return true;
            }
            self.current_grid[row][x] = self.final_grid[row][x].clone();
            *pos += 1;
        }
        false
    }

    fn tick_scanline(&mut self) -> bool {
        let row = match &mut self.effect_state {
            EffectState::Scanline { row } => row,
            _ => return true,
        };
        if *row >= DISPLAY_HEIGHT {
            return true;
        }
        for x in 0..WIDTH {
            self.current_grid[*row][x] = self.final_grid[*row][x].clone();
        }
        *row += 1;
        false
    }

    fn tick_rain(&mut self) -> bool {
        let drops = match &mut self.effect_state {
            EffectState::Rain { drops } => drops,
            _ => return true,
        };

        let mut all_settled = true;
        self.current_grid = create_empty_grid();

        for x in 0..WIDTH {
            let drop = &mut drops[x];

            // Draw settled pixels
            for row in (DISPLAY_HEIGHT.saturating_sub(drop.settled)..DISPLAY_HEIGHT).rev() {
                self.current_grid[row][x] = self.final_grid[row][x].clone();
            }

            if drop.settled >= DISPLAY_HEIGHT {
                continue;
            }
            all_settled = false;

            // Find target row for this column (lowest non-space pixel)
            let mut target_row: i32 = -1;
            for row in (0..DISPLAY_HEIGHT - drop.settled).rev() {
                if self.final_grid[row][x] != " " {
                    target_row = row as i32;
                    break;
                }
            }

            drop.y += 1;

            if drop.y >= 0 && (drop.y as usize) < DISPLAY_HEIGHT {
                if target_row >= 0 && drop.y >= target_row {
                    // Settle
                    drop.settled = DISPLAY_HEIGHT - target_row as usize;
                    drop.y = -1; // reset for next column
                } else {
                    // Still falling
                    self.current_grid[drop.y as usize][x] = "▓".to_string();
                }
            }
        }

        all_settled
    }

    fn tick_fade(&mut self) -> bool {
        let (positions, idx) = match &mut self.effect_state {
            EffectState::Fade { positions, idx } => (positions, idx),
            _ => return true,
        };
        let pixels_per_frame = 15;

        for _ in 0..pixels_per_frame {
            if *idx >= positions.len() {
                return true;
            }
            let (row, x) = positions[*idx];
            self.current_grid[row][x] = self.final_grid[row][x].clone();
            *idx += 1;
        }
        false
    }

    fn tick_crt(&mut self) -> bool {
        let expansion = match &mut self.effect_state {
            EffectState::Crt { expansion } => expansion,
            _ => return true,
        };

        self.current_grid = create_empty_grid();
        let mid_row = DISPLAY_HEIGHT / 2;

        let top = mid_row.saturating_sub(*expansion);
        let bottom = (mid_row + *expansion).min(DISPLAY_HEIGHT - 1);

        for row in top..=bottom {
            for x in 0..WIDTH {
                self.current_grid[row][x] = self.final_grid[row][x].clone();
            }
        }

        *expansion += 1;
        *expansion > DISPLAY_HEIGHT
    }

    fn tick_glitch(&mut self) -> bool {
        let (phase, glitch_frames) = match &mut self.effect_state {
            EffectState::Glitch {
                phase,
                glitch_frames,
            } => (phase, glitch_frames),
            _ => return true,
        };

        if *phase < *glitch_frames {
            // Glitch phase: show corrupted version
            self.current_grid = self.final_grid.clone();
            let mut new_grid = self.current_grid.clone();

            for row in 0..DISPLAY_HEIGHT {
                // Random horizontal offset (30% chance)
                if fast_random() < 0.3 {
                    let offset: i32 = (fast_random() * 7.0) as i32 - 3;
                    let mut shifted = vec![" ".to_string(); WIDTH];
                    for x in 0..WIDTH {
                        let src = ((x as i32 - offset).rem_euclid(WIDTH as i32)) as usize;
                        shifted[x] = self.current_grid[row][src].clone();
                    }
                    new_grid[row] = shifted;
                }
                // Random vertical swap (20% chance)
                if fast_random() < 0.2 {
                    let swap_row =
                        (fast_random() * DISPLAY_HEIGHT as f64) as usize % DISPLAY_HEIGHT;
                    new_grid[row] = self.final_grid[swap_row].clone();
                }
            }
            self.current_grid = new_grid;
            *phase += 1;
            return false;
        }

        // Final frame: show clean image
        self.current_grid = self.final_grid.clone();
        true
    }

    fn tick_dissolve(&mut self) -> bool {
        let (positions, idx) = match &mut self.effect_state {
            EffectState::Dissolve { positions, idx } => (positions, idx),
            _ => return true,
        };
        let pixels_per_frame = 20;

        for _ in 0..pixels_per_frame {
            if *idx >= positions.len() {
                return true;
            }
            let (row, x) = positions[*idx];
            self.current_grid[row][x] = self.final_grid[row][x].clone();
            *idx += 1;
        }
        false
    }
}

impl Component for ArminComponent {
    fn render(&self, width: u16) -> Vec<String> {
        // Due to &self constraints, we recompute each render.
        // In the real TUI integration this uses interior mutability.
        let mut lines: Vec<String> = Vec::new();
        let padding: u16 = 1;
        let available_width = width.saturating_sub(padding);

        for row in &self.current_grid {
            let clipped: String = row.iter().take(available_width as usize).cloned().collect();
            let pad_right =
                (width as isize - padding as isize - clipped.len() as isize).max(0) as usize;
            lines.push(format!(
                " {}{}",
                theme().fg("accent", &clipped),
                " ".repeat(pad_right)
            ));
        }

        // Add "ARMIN SAYS HI" at the end
        let message = "ARMIN SAYS HI";
        let msg_pad_right =
            (width as isize - padding as isize - message.len() as isize).max(0) as usize;
        lines.push(format!(
            " {}{}",
            theme().fg("accent", message),
            " ".repeat(msg_pad_right)
        ));

        lines
    }

    fn invalidate(&mut self) {
        self.cached_width = 0;
        self.cached_version = usize::MAX;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_empty_grid() -> Vec<Vec<String>> {
    vec![vec![" ".to_string(); WIDTH]; DISPLAY_HEIGHT]
}

fn init_effect_state(effect: &EffectType, _final_grid: &[Vec<String>]) -> EffectState {
    match effect {
        EffectType::Typewriter => EffectState::Typewriter { pos: 0 },
        EffectType::Scanline => EffectState::Scanline { row: 0 },
        EffectType::Rain => {
            let mut drops = Vec::with_capacity(WIDTH);
            let _seed = fast_random_seed();
            for _ in 0..WIDTH {
                drops.push(RainDrop {
                    y: -((fast_random_with_seed(&mut fast_random_seed())
                        * (DISPLAY_HEIGHT * 2) as f64) as i32
                        + 1),
                    settled: 0,
                });
            }
            EffectState::Rain { drops }
        }
        EffectType::Fade => {
            let mut positions = Vec::new();
            for row in 0..DISPLAY_HEIGHT {
                for x in 0..WIDTH {
                    positions.push((row, x));
                }
            }
            shuffle(&mut positions);
            EffectState::Fade { positions, idx: 0 }
        }
        EffectType::Crt => EffectState::Crt { expansion: 0 },
        EffectType::Glitch => EffectState::Glitch {
            phase: 0,
            glitch_frames: 8,
        },
        EffectType::Dissolve => {
            // Start with random noise
            let mut positions = Vec::new();
            for row in 0..DISPLAY_HEIGHT {
                for x in 0..WIDTH {
                    positions.push((row, x));
                }
            }
            shuffle(&mut positions);
            EffectState::Dissolve { positions, idx: 0 }
        }
    }
}

/// Simple LCG pseudo-random number generator returning f64 in [0,1).
fn fast_random_with_seed(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as f64 / (1u64 << 31) as f64
}

fn fast_random() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(123456789);
    let mut seed = SEED.load(Ordering::Relaxed);
    seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    SEED.store(seed, Ordering::Relaxed);
    (seed >> 33) as f64 / (1u64 << 31) as f64
}

fn fast_random_seed() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(987654321);
    SEED.load(Ordering::Relaxed)
}

/// Fisher-Yates shuffle.
fn shuffle<T>(v: &mut Vec<T>) {
    let len = v.len();
    for i in (1..len).rev() {
        let j = (fast_random() * (i + 1) as f64) as usize;
        v.swap(i, j);
    }
}
