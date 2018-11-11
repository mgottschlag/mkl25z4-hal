use crate::mkl25z4::SIM;

pub fn disable(sim: &mut SIM) {
    sim.copc
        .write(|w| w.copt()._00().copclks().clear_bit().copw().clear_bit());
}
