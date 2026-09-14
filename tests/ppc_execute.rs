use vasjit::frontend_wii_ppc::{self as ppc, WiiFrontend};
use vasjit::jit_ffi;
use vasjit::runtime::GuestSystem;

fn run_words(words:&[u32])->GuestSystem<WiiFrontend>{
    let caps=jit_ffi::detect_host_caps();
    let mut sys=GuestSystem::new(WiiFrontend,caps,true).expect("mmap");
    let mut bytes=Vec::new();
    for w in words{ bytes.extend_from_slice(&w.to_be_bytes()); }
    bytes.extend_from_slice(&ppc::enc::blr().to_be_bytes());
    sys.write_guest(0x8000_0000,&bytes);
    sys.cpu.pc=0x8000_0000; sys.cpu.lr=0x8000_0100;
    sys.run_steps(4); sys
}

#[test] fn li_r3_42(){
    let sys=run_words(&[ppc::enc::addi(3,0,42)]);
    assert_eq!(sys.cpu.gpr[3],42,"pc={:#x}",sys.cpu.pc);
}
#[test] fn add_r3(){
    let sys=run_words(&[ppc::enc::addi(3,0,5),ppc::enc::add(3,3,3)]);
    assert_eq!(sys.cpu.gpr[3],10);
}
#[test] fn ori_copy(){
    let sys=run_words(&[ppc::enc::addi(4,0,7),ppc::enc::ori(3,4,0)]);
    assert_eq!(sys.cpu.gpr[3],7);
}
#[test] fn stw_lwz(){
    let sys=run_words(&[ppc::enc::addi(3,0,99),ppc::enc::stw(3,0,0x100),ppc::enc::lwz(5,0,0x100)]);
    assert_eq!(sys.cpu.gpr[5],99,"r3={} r5={}",sys.cpu.gpr[3],sys.cpu.gpr[5]);
}
