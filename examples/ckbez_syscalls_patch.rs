use ckb_types::prelude::IntoTransactionView;
use ckb_vm::Register;

pub struct SyscallCurrentCycles {}

impl SyscallCurrentCycles {
    pub fn new() -> Self {
        Self {}
    }
}

impl<Mac: ckb_vm::SupportMachine> ckb_vm::Syscalls<Mac> for SyscallCurrentCycles {
    fn initialize(&mut self, _machine: &mut Mac) -> Result<(), ckb_vm::Error> {
        Ok(())
    }

    fn ecall(&mut self, machine: &mut Mac) -> Result<bool, ckb_vm::Error> {
        let id = machine.registers()[ckb_vm::registers::A7].to_u64();
        if id != 2042 {
            return Ok(false);
        }
        machine.set_register(ckb_vm::registers::A0, Mac::REG::from_u64(2042));
        return Ok(true);
    }
}

pub fn generate_ckb_syscalls_patch(
    vm_id: &ckb_script::types::VmId,
    sg_data: &ckb_script::types::SgData<ckbez::unittest::Resource>,
    vm_context: &ckb_script::types::VmContext<ckbez::unittest::Resource>,
    debug_printer: &ckb_script::types::DebugPrinter,
) -> Vec<Box<(dyn ckb_vm::Syscalls<<ckb_script::types::Machine as ckb_vm::DefaultMachineRunner>::Inner>)>> {
    let mut sys_patch = ckb_script::generate_ckb_syscalls(vm_id, sg_data, vm_context, &debug_printer);
    sys_patch.insert(0, Box::new(SyscallCurrentCycles::new()));
    return sys_patch;
}

fn main() {
    let exit_0 = std::fs::read("res/syscall").unwrap();
    let mut dl = ckbez::unittest::Resource::default();
    let mut px = ckbez::unittest::Pickaxer::default();

    let mut tx = ckbez::core::Transaction::default();
    let cell_meta_lock = px.create_cell(&mut dl, 0, ckbez::core::Script::default(), None, &exit_0);
    let cell_meta_i = px.create_cell(&mut dl, 0, px.create_script_by_data(&cell_meta_lock, &[]), None, &[]);
    tx.raw.cell_deps.push(px.create_cell_dep(&cell_meta_lock, 0));
    tx.raw.inputs.push(px.create_cell_input(&cell_meta_i));
    let tx_view = tx.pack().into_view();

    let config =
        ckb_script::config::Config::devnet().as_builder().syscall_generator(generate_ckb_syscalls_patch).build();
    let verify = config.transaction_scripts_verifier(tx_view, dl).unwrap();
    let script_hash = ckb_types::packed::Byte32::new(cell_meta_i.cell_output.lock.hash());
    assert_eq!(
        verify.get_script_hash_by_location("input".parse().unwrap(), 0, "lock".parse().unwrap()).unwrap(),
        script_hash,
    );
    let result = verify.verify_single("lock".parse().unwrap(), &script_hash, 70_000_000);
    println!("verify_single {:?}", result);
}
