#![no_std]
#![no_main]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use cortex_m_rt::entry;
use cortex_m_semihosting::{hprintln, debug};
use embedded_alloc::LlffHeap;

// ── Tracking allocator ────────────────────────────────────────────────────────
// Wraps LlffHeap and records peak live-bytes at any point in time.

struct TrackingHeap(LlffHeap);

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK:    AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.0.alloc(layout) };
        if !ptr.is_null() {
            let prev = CURRENT.fetch_add(layout.size(), Ordering::Relaxed);
            let new  = prev + layout.size();
            let mut peak = PEAK.load(Ordering::Relaxed);
            while new > peak {
                match PEAK.compare_exchange_weak(peak, new, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_)  => break,
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

#[global_allocator]
static HEAP: TrackingHeap = TrackingHeap(LlffHeap::empty());

const HEAP_SIZE: usize = 64 * 1024;
static mut HEAP_MEM: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];

// ── Critical section ──────────────────────────────────────────────────────────

struct CortexMCs;
critical_section::set_impl!(CortexMCs);

unsafe impl critical_section::Impl for CortexMCs {
    unsafe fn acquire() -> () {
        unsafe { core::arch::asm!("cpsid i", options(nomem, nostack)); }
    }
    unsafe fn release(_: ()) {
        unsafe { core::arch::asm!("cpsie i", options(nomem, nostack)); }
    }
}

// ── Stack high-water mark via cortex-m-rt stack symbol ───────────────────────
// cortex-m-rt places _stack_start at the top of RAM and grows down.
// We fill from __ebss (end of static data) up to _stack_start with 0xAB,
// then scan to find how far down the stack reached.

const STACK_CANARY: u8 = 0xAB;

unsafe extern "C" {
    static mut __ebss: u32;        // end of .bss  (bottom of stack region)
    static _stack_start: u32;      // top of RAM    (initial SP)
}

fn fill_stack_canary() {
    unsafe {
        let bottom = core::ptr::addr_of_mut!(__ebss) as *mut u8;
        let top    = core::ptr::addr_of!(_stack_start) as *mut u8;
        // Leave 256 bytes at the top so the current stack frame survives
        let safe_top = top.sub(256);
        let mut p = bottom;
        while p < safe_top {
            p.write_volatile(STACK_CANARY);
            p = p.add(1);
        }
    }
}

fn stack_hwm() -> usize {
    unsafe {
        let bottom = core::ptr::addr_of!(__ebss) as *const u8;
        let top    = core::ptr::addr_of!(_stack_start) as *const u8;
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

fn run_case(expr: &str) {
    use calc_core::{parse, simplify};
    if let Ok(e) = parse(expr) {
        let s = simplify(e);
        core::hint::black_box(s);
    }
}

fn run_all() -> usize {
    let cases: &[&str] = &[
        // Arithmetic / rational
        "1 + 2 + 3 + 4 + 5",
        "100 / 7 + 3/14",
        "2^10",
        "2^32",
        "1000000 * 999999",
        // Implicit multiply
        "2x",
        "3pi",
        // Trig
        "sin(pi/6)",
        "cos(0)",
        "tan(pi/4)",
        "asin(1)",
        "acos(0)",
        "atan(1)",
        // Hyperbolic
        "sinh(0)",
        "cosh(0)",
        "tanh(0)",
        // Exp / log
        "exp(0)",
        "ln(1)",
        "log(10, 100)",
        "log2(8)",
        "log10(1000)",
        // Roots
        "sqrt(144)",
        "cbrt(27)",
        // Pythagorean identity
        "sin(x)^2 + cos(x)^2",
        // Inverse cancellation
        "exp(ln(x))",
        "ln(exp(x))",
        "sqrt(x^2)",
        // Polynomial
        "expand((x+1)^3)",
        "expand((x+2)(x-2))",
        "expand((x+y)^2)",
        // Derivatives
        "diff(x^3, x)",
        "diff(sin(x), x)",
        "diff(exp(x), x)",
        "diff(ln(x), x)",
        "diff(x^3, x, 2)",
        // Integration
        "integrate(x^2, x)",
        "integrate(sin(x), x)",
        "integrate(exp(x), x)",
        // Solve
        "solve(x^2 - 4, x)",
        "solve(2x + 6, x)",
        "solve(x^2 == 9, x)",
        // Taylor series
        "taylor(sin(x), x, 0, 5)",
        "taylor(exp(x), x, 0, 4)",
        // Number theory
        "gcd(48, 18)",
        "lcm(12, 15)",
        "5!",
        "isprime(97)",
        // Sequences
        "sum(x, x, 1, 10)",
        "product(x, x, 1, 5)",
        "range(8)",
        // Matrix constructors
        "zeros(3)",
        "ones(2)",
        "eye(3)",
        // Matrix ops
        "det([[1,2],[3,4]])",
        "tr([[1,0,0],[0,2,0],[0,0,3]])",
        "transpose([[1,2,3],[4,5,6]])",
        "dot([3,4], [3,4])",
        "norm([3,4])",
        // Complex
        "re(3 + 4i)",
        "im(3 + 4i)",
        // Nested / deep
        "diff(expand((x^2 + 2x + 1)), x)",
        "simplify(sin(x)^2 + cos(x)^2 + 1)",
    ];
    for c in cases { run_case(c); }
    cases.len()
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[entry]
fn main() -> ! {
    unsafe { HEAP.0.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }

    fill_stack_canary();

    let n = run_all();

    let peak_heap  = PEAK.load(Ordering::Relaxed);
    let stack_used = stack_hwm();

    hprintln!("=== opencalc embedded stress results ===");
    hprintln!("cases run      : {}", n);
    hprintln!("peak heap      : {} bytes  ({} kB)", peak_heap, peak_heap / 1024);
    hprintln!("heap headroom  : {} bytes  ({} kB)", HEAP_SIZE - peak_heap, (HEAP_SIZE - peak_heap) / 1024);
    hprintln!("stack hwm      : {} bytes", stack_used);

    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
