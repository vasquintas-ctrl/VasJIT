use super::GenericFrame;
pub fn translate_frame(stream:&[u8])->GenericFrame{
    let mut draws=0usize; let mut i=0;
    while i+4<=stream.len(){
        let hdr=u32::from_le_bytes([stream[i],stream[i+1],stream[i+2],stream[i+3]]);
        i+=4; let count=((hdr>>16)&0x1fff) as usize; let method=hdr&0x1fff;
        if method==0x0d74 || method==0x0d80 { draws+=1; }
        i=i.saturating_add(count.saturating_mul(4));
    }
    GenericFrame{width:1280,height:720,draws}
}
