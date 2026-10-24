//! Icono de plantilla (monitor) dibujado en código: sin archivos externos.

pub fn tray_icon() -> tray_icon::Icon {
    const W: u32 = 36;
    const H: u32 = 36;
    let mut px = vec![0u8; (W * H * 4) as usize];
    let mut set = |x: u32, y: u32| {
        let i = ((y * W + x) * 4) as usize;
        px[i..i + 4].copy_from_slice(&[0, 0, 0, 255]);
    };
    // Marco de la pantalla
    for x in 3..33 {
        for y in 6..9 { set(x, y); }
        for y in 23..26 { set(x, y); }
    }
    for y in 6..26 {
        for x in 3..6 { set(x, y); }
        for x in 30..33 { set(x, y); }
    }
    // Pie
    for y in 26..30 { for x in 16..20 { set(x, y); } }
    for y in 30..32 { for x in 10..26 { set(x, y); } }
    tray_icon::Icon::from_rgba(px, W, H).expect("icono")
}
