use super::GenericFrame;
pub fn translate_frame(bp_writes:&[u32])->GenericFrame{
    let mut w=640u32; let mut h=480u32; let mut stages=1usize;
    for &word in bp_writes{
        let addr=(word>>24) as u8; let val=word&0x00ff_ffff;
        if addr==0{ stages=(((val>>10)&0xf)+1) as usize; }
        if addr==0x20{ /* scissor */ let _=val; }
        if addr==0x21{ w=((val&0xfff).saturating_sub(0)).max(1); h=(((val>>12)&0xfff)).max(1); }
    }
    GenericFrame{width:w,height:h,draws:stages}
}
