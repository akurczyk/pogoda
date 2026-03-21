use std::io::{self, Write as IoWrite};

pub fn print_banner(
    out: &mut impl IoWrite,
    main_rgb: (u8, u8, u8),
    shadow_rgb: (u8, u8, u8),
    mono: bool,
) -> io::Result<()> {
    // 5-row pixel font, 4 cols per letter; 1=filled (██) 0=empty
    // P         O         G         O         D         A
    let font: &[&[u8]] = &[
        &[0b1110, 0b1001, 0b1110, 0b1000, 0b1000],
        &[0b0110, 0b1001, 0b1001, 0b1001, 0b0110],
        &[0b0111, 0b1000, 0b1011, 0b1001, 0b0111],
        &[0b0110, 0b1001, 0b1001, 0b1001, 0b0110],
        &[0b1110, 0b1001, 0b1001, 0b1001, 0b1110],
        &[0b0110, 0b1001, 0b1111, 0b1001, 0b1001],
    ];

    let pixel = |letter: &[u8], row: usize, col: usize| -> bool {
        row < 5 && col < 4 && (letter[row] >> (3 - col)) & 1 == 1
    };
    let (mr, mg, mb) = main_rgb;
    let (sr, sg, sb) = shadow_rgb;

    for drow in 0..5usize {
        write!(out, "  ")?;
        for letter in font {
            for dcol in 0..5usize {
                let main = pixel(letter, drow, dcol);
                let shadow = dcol > 0 && pixel(letter, drow, dcol - 1);
                if main {
                    if mono {
                        write!(out, "██")?;
                    } else {
                        write!(out, "\x1b[38;2;{mr};{mg};{mb}m██\x1b[0m")?;
                    }
                } else if shadow {
                    if mono {
                        write!(out, "█ ")?;
                    } else {
                        write!(out, "\x1b[38;2;{sr};{sg};{sb}m█\x1b[0m ")?;
                    }
                } else {
                    write!(out, "  ")?;
                }
            }
            write!(out, "  ")?;
        }
        writeln!(out)?;
    }
    writeln!(out)?;
    Ok(())
}
