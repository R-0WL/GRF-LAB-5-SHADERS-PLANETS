//! HUD espacial: tipografia plateada y emblema de nave dibujado en codigo.
use crate::framebuffer::Framebuffer;

pub fn rect(f: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, color: u32) {
    for j in y..(y + h).min(f.height) {
        for i in x..(x + w).min(f.width) {
            f.buffer[j * f.width + i] = color;
        }
    }
}

// Silver, spaced, geometric capitals: visual treatment inspired by MIB.
const TITLE_FONT: &[u8] = include_bytes!("../assets/title_font.alpha");
const TITLE_WIDTHS: &[u8] = include_bytes!("../assets/title_font.widths");
fn title_text(f: &mut Framebuffer, value: &str, x: usize, y: usize, height: usize, available: usize) {
    let chars: Vec<usize> = value.to_ascii_uppercase().chars().map(|ch| {
        if (' '..='~').contains(&ch) {ch as usize-32} else {0}
    }).collect();
    let units: usize = chars.iter().map(|i| TITLE_WIDTHS[*i] as usize+4).sum();
    let scale = (height as f32/30.0).min(available as f32/units.max(1) as f32);
    let mut cursor=x as f32;
    for index in chars {
        let gw=TITLE_WIDTHS[index] as usize;
        let dw=((gw as f32*scale).ceil() as usize).max(1);
        let dh=((40.0*scale).ceil() as usize).max(1);
        for dy in 0..dh { for dx in 0..dw {
            let sx=((dx as f32/scale) as usize).min(31);
            let sy=((dy as f32/scale) as usize).min(39);
            let alpha=TITLE_FONT[sy*(32*95)+index*32+sx] as f32/255.0;
            if alpha>0.0 {
                let shade=(246.0-dy as f32/dh as f32*76.0) as u32;
                f.set_current_color((shade<<16)|((shade+5).min(255)<<8)|(shade+9).min(255));
                f.blend_point(cursor as i32+dx as i32,y as i32+dy as i32,alpha);
            }
        }}
        cursor+=(gw+4) as f32*scale;
    }
}
/// Small code-drawn spacecraft emblem, independent of platform emoji fonts.
pub fn ship(f: &mut Framebuffer, x: usize, y: usize, size: usize) {
    for py in 0..size { for px in 0..size {
        let u=(px as f32+0.5)/size.max(1) as f32*2.0-1.0;
        let v=(py as f32+0.5)/size.max(1) as f32;
        let hull=u.abs()<0.11+0.20*v && v<0.83;
        let wings=v>0.38 && v<0.88 && u.abs()<(v-0.25)*1.35 && u.abs()>0.18;
        let engine=v>0.77 && u.abs()<0.14*(1.0-v)/0.23;
        if hull||wings||engine {
            let color=if engine {0x39CFFF} else if hull && u.abs()<0.08 && v>0.25 && v<0.52 {0x57DAFF} else {0xDCE8F4};
            rect(f,x+px,y+py,1,1,color);
        }
    }}
}
pub fn heading(f: &mut Framebuffer, body: &str, layers: u8) {
    let compact=f.width<360;
    let height=if compact {10} else if f.width<800 {18} else {25};
    let icon=if compact {17} else {29};
    let margin=if compact {6} else {14};
    let bar=if compact {31} else {58};
    let width=f.width;
    rect(f,0,0,width,bar,0x050913);
    ship(f,margin,if compact {6} else {12},icon);
    let x=margin+icon+10;
    title_text(f,body,x,if compact {4} else {8},height,width.saturating_sub(x+margin));
    title_text(f,&format!("CAPAS 1-{}",layers),x,if compact {18} else {36},if compact {7} else {10},width.saturating_sub(x+margin));
    rect(f,margin,bar-1,width.saturating_sub(margin*2),1,0x28485E);
}

pub fn caption(f: &mut Framebuffer, value: &str, x: usize, y: usize) {
    let available=f.width.saturating_sub(x+12);
    title_text(f,value,x,y,10,available);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headings_fit_small_frames_and_include_the_spaceship() {
        assert_eq!(TITLE_FONT.len(),32*95*40);
        assert_eq!(TITLE_WIDTHS.len(),95);
        let mut frame=Framebuffer::new(160,80);
        heading(&mut frame,"SOL / LUIS MIGUEL",4);
        assert!(frame.buffer.chunks(160).take(25).any(|row| row[6..23].iter().any(|p|*p==0x57DAFF)));
        assert!(frame.buffer.iter().any(|p|*p>0xA0A0A0));
    }
}
