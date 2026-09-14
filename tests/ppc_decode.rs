use vasjit::frontend_wii_ppc::{self as ppc, PpcInsn, WiiFrontend};
use vasjit::frontend::Frontend;

fn dec(word:u32)->PpcInsn{ WiiFrontend.decode(&word.to_be_bytes(),0x8000_0000).0.insn }

#[test] fn primary_table(){
    assert!(matches!(dec(ppc::enc::addi(3,1,-4)), PpcInsn::Addi{rd:3,ra:1,simm:-4}));
    assert!(matches!(dec(ppc::enc::add(3,4,5)), PpcInsn::Add{rd:3,ra:4,rb:5,..}));
    assert!(matches!(dec(ppc::enc::ori(3,4,0x10)), PpcInsn::Ori{..}));
    assert!(matches!(dec(ppc::enc::lwz(3,1,8)), PpcInsn::Lwz{rd:3,ra:1,d:8,update:false}));
}
#[test] fn opcode19_split(){
    assert!(matches!(dec(ppc::enc::blr()), PpcInsn::Bclr{..}));
    let crand=(19u32<<26)|(257<<1);
    assert!(matches!(dec(crand), PpcInsn::Crand{..}));
    assert!(!dec(crand).is_terminator());
}
#[test] fn mask_rlwinm(){
    assert_eq!(ppc::mask_mbme(0,31),0xffff_ffff);
    assert_eq!(ppc::mask_mbme(16,31),0x0000_ffff);
}
