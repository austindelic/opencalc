#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

extern crate alloc;

#[cfg(not(test))]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(not(test))]
use core::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(test))]
use cortex_m_rt::entry;
#[cfg(not(test))]
use cortex_m_semihosting::{debug, hprintln};
#[cfg(not(test))]
use embedded_alloc::LlffHeap;

// ── Tracking allocator ────────────────────────────────────────────────────────
// Wraps LlffHeap and records peak live-bytes at any point in time.

#[cfg(not(test))]
struct TrackingHeap(LlffHeap);

#[cfg(not(test))]
static CURRENT: AtomicUsize = AtomicUsize::new(0);
#[cfg(not(test))]
static PEAK: AtomicUsize = AtomicUsize::new(0);

#[cfg(not(test))]
unsafe impl GlobalAlloc for TrackingHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.0.alloc(layout) };
        if !ptr.is_null() {
            let prev = CURRENT.fetch_add(layout.size(), Ordering::Relaxed);
            let new = prev + layout.size();
            let mut peak = PEAK.load(Ordering::Relaxed);
            while new > peak {
                match PEAK.compare_exchange_weak(peak, new, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[cfg(not(test))]
#[global_allocator]
static HEAP: TrackingHeap = TrackingHeap(LlffHeap::empty());

#[cfg(not(test))]
const HEAP_SIZE: usize = 64 * 1024;
#[cfg(not(test))]
static mut HEAP_MEM: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];

// ── Critical section ──────────────────────────────────────────────────────────

#[cfg(not(test))]
struct CortexMCs;
#[cfg(not(test))]
critical_section::set_impl!(CortexMCs);

#[cfg(not(test))]
unsafe impl critical_section::Impl for CortexMCs {
    unsafe fn acquire() -> () {
        unsafe {
            core::arch::asm!("cpsid i", options(nomem, nostack));
        }
    }
    unsafe fn release(_: ()) {
        unsafe {
            core::arch::asm!("cpsie i", options(nomem, nostack));
        }
    }
}

// ── Stack high-water mark via cortex-m-rt stack symbol ───────────────────────
// cortex-m-rt places _stack_start at the top of RAM and grows down.
// We fill from __ebss (end of static data) up to _stack_start with 0xAB,
// then scan to find how far down the stack reached.

#[cfg(not(test))]
const STACK_CANARY: u8 = 0xAB;

#[cfg(not(test))]
unsafe extern "C" {
    static mut __ebss: u32; // end of .bss  (bottom of stack region)
    static _stack_start: u32; // top of RAM    (initial SP)
}

#[cfg(not(test))]
fn fill_stack_canary() {
    unsafe {
        let bottom = core::ptr::addr_of_mut!(__ebss) as *mut u8;
        let top = core::ptr::addr_of!(_stack_start) as *mut u8;
        // Leave 256 bytes at the top so the current stack frame survives
        let safe_top = top.sub(256);
        let mut p = bottom;
        while p < safe_top {
            p.write_volatile(STACK_CANARY);
            p = p.add(1);
        }
    }
}

#[cfg(not(test))]
fn stack_hwm() -> usize {
    unsafe {
        let bottom = core::ptr::addr_of!(__ebss) as *const u8;
        let top = core::ptr::addr_of!(_stack_start) as *const u8;
        let region = top as usize - bottom as usize;
        // Scan from bottom; first byte != canary = lowest stack address touched
        let mut touched_at = region;
        let mut p = bottom;
        while p < top {
            if p.read_volatile() != STACK_CANARY {
                touched_at = p as usize - bottom as usize;
                break;
            }
            p = p.add(1);
        }
        region - touched_at
    }
}

// ── Stress cases ──────────────────────────────────────────────────────────────

#[cfg(not(test))]
fn run_case(expr: &str) {
    use calc_core::{parse, simplify};
    if let Ok(e) = parse(expr) {
        let s = simplify(e);
        core::hint::black_box(s);
    }
}

#[cfg(not(test))]
fn run_all() -> usize {
    #[cfg(feature = "qemu-selftest")]
    {
        calc_core::selftest::run_all().total
    }

    #[cfg(not(feature = "qemu-selftest"))]
    for c in calc_core::tests::CALCULATOR_CONFORMANCE_CASES {
        run_case(c);
    }
    #[cfg(not(feature = "qemu-selftest"))]
    {
        calc_core::tests::CALCULATOR_CONFORMANCE_CASES.len()
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[cfg(not(test))]
#[entry]
fn main() -> ! {
    unsafe {
        HEAP.0
            .init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE)
    }

    fill_stack_canary();

    #[cfg(feature = "qemu-selftest")]
    let report = calc_core::selftest::run_all();
    #[cfg(feature = "qemu-selftest")]
    let n = report.total;
    #[cfg(not(feature = "qemu-selftest"))]
    let n = run_all();

    let peak_heap = PEAK.load(Ordering::Relaxed);
    let stack_used = stack_hwm();

    #[cfg(feature = "qemu-selftest")]
    hprintln!("=== opencalc embedded self-test results ===");
    #[cfg(not(feature = "qemu-selftest"))]
    hprintln!("=== opencalc embedded stress results ===");

    #[cfg(feature = "qemu-selftest")]
    {
        hprintln!("tests passed   : {}/{}", report.passed, report.total);
        hprintln!("tests failed   : {}", report.failures.len());
        for failure in report.failures.iter().take(8) {
            hprintln!("failure        : {}", failure);
        }
    }
    #[cfg(not(feature = "qemu-selftest"))]
    hprintln!("cases run      : {}", n);
    hprintln!(
        "peak heap      : {} bytes  ({} kB)",
        peak_heap,
        peak_heap / 1024
    );
    hprintln!(
        "heap headroom  : {} bytes  ({} kB)",
        HEAP_SIZE - peak_heap,
        (HEAP_SIZE - peak_heap) / 1024
    );
    hprintln!("stack hwm      : {} bytes", stack_used);

    #[cfg(feature = "qemu-selftest")]
    if !report.failures.is_empty() {
        debug::exit(debug::EXIT_FAILURE);
        loop {}
    }

    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    use calc_core::ScriptRuntime;

    #[test]
    fn rhai_script_should_call_core_calculator_bridge() {
        let scripts = ScriptRuntime::new();
        let result = scripts
            .run(r#"calc("diff(x^3, x)") + " | " + calc("sqrt(144)")"#)
            .unwrap();

        assert_eq!(result, "3·x^2 | 12");
    }

    #[test]
    fn compiled_rhai_script_should_run_repeatedly_with_scope() {
        let scripts = ScriptRuntime::new();
        let compiled = scripts
            .compile(r#"count += 1; calc("2^" + count) + ":" + count"#)
            .unwrap();
        let mut scope = ScriptRuntime::new_scope();
        scope.push("count", 0_i64);

        assert_eq!(
            scripts
                .run_compiled_with_scope(&compiled, &mut scope)
                .unwrap(),
            "2:1"
        );
        assert_eq!(
            scripts
                .run_compiled_with_scope(&compiled, &mut scope)
                .unwrap(),
            "4:2"
        );
    }

    #[test]
    fn rhai_script_should_stress_calculator_calls_in_loop() {
        let scripts = ScriptRuntime::new();
        let result = scripts
            .run(
                r#"
                let total = 0.0;
                for n in 1..=6 {
                    total += value("2^" + n);
                }
                total
                "#,
            )
            .unwrap();

        assert_eq!(result, "126.0");
    }
}
