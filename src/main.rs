use vasjit::frontend::Frontend;
use vasjit::frontend_wii_ppc::{self, WiiFrontend};
use vasjit::frontend_switch_arm64::SwitchFrontend;
use vasjit::ir::IrBuilder;
use vasjit::jit_ffi;
use vasjit::runtime::GuestSystem;

fn main(){
    let args:Vec<String>=std::env::args().collect();
    let cmd=args.get(1).map(String::as_str).unwrap_or("help");
    match cmd{
        "caps"=>cmd_caps(),
        "decode-ppc"|"ppc"=>cmd_decode_ppc(&args[2..]),
        "decode-a64"|"a64"=>cmd_decode_a64(&args[2..]),
        "lower-ppc"=>cmd_lower_ppc(&args[2..]),
        "smoke"=>cmd_smoke(),
        _=>{
            eprintln!("VasJIT — Broadway / Tegra X1 JIT\n");
            eprintln!("  vasjit caps");
            eprintln!("  vasjit decode-ppc <hex words...>");
            eprintln!("  vasjit lower-ppc  <hex words...>");
            eprintln!("  vasjit decode-a64 <hex words...>");
            eprintln!("  vasjit smoke");
        }
    }
}
fn cmd_caps(){
    let caps=jit_ffi::detect_host_caps();
    println!("host caps bits = {:#010x}", caps.bits);
    println!("  SSE2    {}", caps.has(vasjit::CHostCaps::SSE2));
    println!("  AVX2    {}", caps.has(vasjit::CHostCaps::AVX2));
    println!("  FMA     {}", caps.has(vasjit::CHostCaps::FMA));
    println!("  BMI2    {}", caps.has(vasjit::CHostCaps::BMI2));
    println!("  MOVBE   {}", caps.has(vasjit::CHostCaps::MOVBE));
}
fn parse_words(args:&[String])->Vec<u32>{
    args.iter().filter_map(|s| u32::from_str_radix(s.trim_start_matches("0x"),16).ok()).collect()
}
fn cmd_decode_ppc(args:&[String]){
    let f=WiiFrontend; let mut pc=0x8000_0000u64;
    for w in parse_words(args){ let (d,_)=f.decode(&w.to_be_bytes(),pc); println!("{}", d.disasm(pc)); pc+=4; }
}
fn cmd_decode_a64(args:&[String]){
    let f=SwitchFrontend; let mut pc=0u64;
    for w in parse_words(args){ let (d,_)=f.decode(&w.to_le_bytes(),pc); println!("{pc:016x}:  {w:08x}  {}", d.insn.mnemonic()); pc+=4; }
}
fn cmd_lower_ppc(args:&[String]){
    let f=WiiFrontend; let mut builder=IrBuilder::new(); builder.endian=vasjit::ir::Endian::Big;
    let block=builder.new_block(0x8000_0000); let mut pc=0x8000_0000u64;
    for w in parse_words(args){
        let (d,_)=f.decode(&w.to_be_bytes(),pc); builder.current_pc=pc;
        println!("# {}", d.disasm(pc)); f.lower_to_ir(&d,&mut builder,block); pc+=4;
    }
    for op in &builder.blocks[0].ops{ println!("  {op}"); }
}
fn cmd_smoke(){
    let caps=jit_ffi::detect_host_caps();
    let force=std::env::var_os("FORCE_BASELINE_CODEGEN").is_some();
    let mut sys=GuestSystem::new(WiiFrontend,caps,force).expect("guest memory");
    let words=[frontend_wii_ppc::enc::addi(3,0,42), frontend_wii_ppc::enc::blr()];
    let mut bytes=Vec::new(); for w in words{ bytes.extend_from_slice(&w.to_be_bytes()); }
    sys.write_guest(0x8000_0000,&bytes);
    sys.cpu.pc=0x8000_0000; sys.cpu.lr=0x8000_0008;
    sys.run_steps(1);
    println!("r3  = {}", sys.cpu.gpr[3]);
    println!("pc  = {:#x}", sys.cpu.pc);
    println!("ok  = {}", sys.cpu.gpr[3]==42);
    if sys.cpu.gpr[3]!=42{ std::process::exit(1); }
}
