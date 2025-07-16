use ckb_types::prelude::IntoTransactionView;

fn main() {
    let exit_0 = std::fs::read("res/exit_0").unwrap();
    let mut dl = ckbez::unittest::Resource::default();
    let mut px = ckbez::unittest::Pickaxer::default();

    let mut tx = ckbez::core::Transaction::default();
    let cell_meta_lock = px.create_cell(&mut dl, 0, ckbez::core::Script::default(), None, &exit_0);
    let cell_meta_i = px.create_cell(&mut dl, 0, px.create_script_by_data(&cell_meta_lock, &[]), None, &[]);
    tx.raw.cell_deps.push(px.create_cell_dep(&cell_meta_lock, 0));
    tx.raw.inputs.push(px.create_cell_input(&cell_meta_i));
    let tx_view = tx.pack().into_view();

    let config: ckb_script::config::Config<_, _, ckb_script::types::Machine> = ckb_script::config::Config::devnet();
    let verify = config.transaction_scripts_verifier(tx_view, dl).unwrap();
    let script_hash = ckb_types::packed::Byte32::new(cell_meta_i.cell_output.lock.hash());
    let result = verify.verify_single("lock".parse().unwrap(), &script_hash, 70_000_000);
    println!("verify_single {:?}", result);
}
