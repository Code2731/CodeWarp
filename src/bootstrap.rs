pub(crate) fn build_window_icon() -> Option<iced::window::Icon> {
    const SIZE: u32 = 64;
    const CENTER: f32 = (SIZE as f32 - 1.0) / 2.0;
    const RADIUS: f32 = 27.0;
    const INNER_RADIUS: f32 = 16.0;
    let mut pixels: Vec<u8> = Vec::with_capacity((SIZE * SIZE * 4) as usize);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - CENTER;
            let dy = y as f32 - CENTER;
            let distance = (dx * dx + dy * dy).sqrt();

            let (r, g, b, a) = if distance <= RADIUS {
                if distance <= INNER_RADIUS {
                    (250, 250, 255, 255)
                } else {
                    let t = 1.0 - (distance - INNER_RADIUS) / (RADIUS - INNER_RADIUS);
                    (
                        (40.0 + t * 120.0) as u8,
                        (80.0 + t * 60.0) as u8,
                        (180.0 + t * 55.0) as u8,
                        255,
                    )
                }
            } else {
                (0, 0, 0, 0)
            };

            pixels.extend_from_slice(&[r, g, b, a]);
        }
    }

    iced::window::icon::from_rgba(pixels, SIZE, SIZE).ok()
}

pub(crate) const PRETENDARD_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/Pretendard-Regular.otf");
pub(crate) const PRETENDARD_SEMIBOLD: &[u8] =
    include_bytes!("../assets/fonts/Pretendard-SemiBold.otf");
pub(crate) const PRETENDARD_BOLD: &[u8] = include_bytes!("../assets/fonts/Pretendard-Bold.otf");
pub(crate) const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
pub(crate) const JETBRAINS_MONO_BOLD: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");
