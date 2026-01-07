use std::io;
use std::time::Instant;

// -----------------------------------------------------------------------------
// 1. FFI Declaration (The Wiring)
// -----------------------------------------------------------------------------
unsafe extern "C" {
    /// The function defined in wavesort.asm
    /// Signature: void wavesort(int32_t *arr, size_t len);
    fn wave_sort(arr: *mut i32, len: usize);
}

// Safe Rust Wrapper for the ASM function
pub fn wavesort_asm_safe(arr: &mut [i32]) {
    unsafe {
        wave_sort(arr.as_mut_ptr(), arr.len());
    }
}

// -----------------------------------------------------------------------------
// 2. Pure Rust Implementation (For Comparison)
// -----------------------------------------------------------------------------
mod wavesort_rust {
    const INSERTION_THRESHOLD: usize = 32;

    pub fn sort(arr: &mut [i32]) {
        let n = arr.len();
        if n < 2 {
            return;
        }
        if n <= INSERTION_THRESHOLD {
            insertion_sort(arr);
            return;
        }
        upwave(arr, 0, n - 1);
    }

    fn insertion_sort(arr: &mut [i32]) {
        let len = arr.len();
        if len < 2 {
            return;
        }
        for i in 1..len {
            unsafe {
                let key = *arr.get_unchecked(i);
                let mut j = i;
                while j > 0 && *arr.get_unchecked(j - 1) > key {
                    *arr.get_unchecked_mut(j) = *arr.get_unchecked(j - 1);
                    j -= 1;
                }
                *arr.get_unchecked_mut(j) = key;
            }
        }
    }

    #[inline(always)]
    fn block_swap(arr: &mut [i32], m: usize, r: usize, p: usize) {
        let left_len = r.wrapping_sub(m);
        if left_len == 0 {
            return;
        }
        let range_len = p - m + 1;
        arr[m..m + range_len].rotate_left(left_len);
    }

    fn partition(arr: &mut [i32], l: usize, r: usize, p_idx: usize) -> usize {
        unsafe {
            let ptr = arr.as_mut_ptr();
            let pivot_val = *ptr.add(p_idx);
            let mut i = l;
            let mut j = r;
            loop {
                loop {
                    let val = *ptr.add(i);
                    if val >= pivot_val {
                        break;
                    }
                    i += 1;
                    if i == j {
                        return i;
                    }
                }
                loop {
                    if j == i {
                        return i;
                    }
                    j -= 1;
                    let val = *ptr.add(j);
                    if val <= pivot_val {
                        break;
                    }
                }
                std::ptr::swap(ptr.add(i), ptr.add(j));
            }
        }
    }

    fn downwave(arr: &mut [i32], start: usize, sorted_start: usize, end: usize) {
        if sorted_start == start {
            return;
        }
        if end - start <= INSERTION_THRESHOLD {
            insertion_sort(&mut arr[start..=end]);
            return;
        }
        let p = sorted_start + (end - sorted_start) / 2;
        let m = partition(arr, start, sorted_start, p);
        if m == sorted_start {
            if p == sorted_start {
                if sorted_start > 0 {
                    upwave(arr, start, sorted_start - 1);
                }
                return;
            }
            if p > 0 {
                downwave(arr, start, sorted_start, p - 1);
            }
            return;
        }
        block_swap(arr, m, sorted_start, p);
        if m == start {
            if p == sorted_start {
                upwave(arr, m + 1, end);
                return;
            }
            let p_next = p + 1;
            downwave(arr, m + p_next - sorted_start, p_next, end);
            return;
        }
        if p == sorted_start {
            if m > 0 {
                upwave(arr, start, m - 1);
            }
            upwave(arr, m + 1, end);
            return;
        }
        let right_part_len = p - sorted_start;
        let split_point = m + right_part_len;
        if split_point > 0 {
            downwave(arr, start, m, split_point - 1);
        }
        downwave(arr, split_point + 1, p + 1, end);
    }

    fn upwave(arr: &mut [i32], start: usize, end: usize) {
        if start == end {
            return;
        }
        if end - start <= INSERTION_THRESHOLD {
            insertion_sort(&mut arr[start..=end]);
            return;
        }
        let mut sorted_start = end;
        let mut sorted_len;
        if end == 0 {
            return;
        }
        let mut left_bound = end - 1;
        let total_len = end - start + 1;
        loop {
            downwave(arr, left_bound, sorted_start, end);
            sorted_start = left_bound;
            sorted_len = end - sorted_start + 1;
            if total_len < (sorted_len << 2) {
                break;
            }
            let next_expansion = (sorted_len << 1) + 1;
            if end < next_expansion || (end - next_expansion) < start {
                left_bound = start;
            } else {
                left_bound = end - next_expansion;
            }
            if left_bound < start {
                left_bound = start;
            }
            if sorted_start == start {
                break;
            }
        }
        downwave(arr, start, sorted_start, end);
    }
}

fn main() -> io::Result<()> {
    const N: usize = 100_000_000;
    println!("Initializing benchmark for {} integer samples...", N);

    // Generate random data
    let mut data_asm = Vec::with_capacity(N);
    let mut seed: u64 = 1;
    for _ in 0..N {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (seed / 65536) % 2147483648;
        data_asm.push(val as i32);
    }

    // Clone for fair comparison
    let mut data_rust = data_asm.clone();
    let mut data_std = data_asm.clone();

    println!("Data generated. Starting benchmark...\n");

    // --- Rust WaveSort ---
    let start_rust = Instant::now();
    wavesort_rust::sort(&mut data_rust);
    let dur_rust = start_rust.elapsed();
    println!("Rust WaveSort: {:.6} s", dur_rust.as_secs_f64());

    // --- ASM WaveSort ---
    let start_asm = Instant::now();
    wavesort_asm_safe(&mut data_asm);
    let dur_asm = start_asm.elapsed();
    println!("ASM  WaveSort: {:.6} s", dur_asm.as_secs_f64());

    // --- Standard Lib ---
    let start_std = Instant::now();
    data_std.sort();
    let dur_std = start_std.elapsed();
    println!("Std  Sort:     {:.6} s", dur_std.as_secs_f64());

    // --- Verification ---
    if !is_sorted(&data_rust) {
        eprintln!("FAILURE: Rust WaveSort failed.");
    }
    if !is_sorted(&data_asm) {
        eprintln!("FAILURE: ASM WaveSort failed.");
    }
    if !is_sorted(&data_std) {
        eprintln!("FAILURE: Std Sort failed.");
    }

    Ok(())
}

fn is_sorted(arr: &[i32]) -> bool {
    arr.windows(2).all(|w| w[0] <= w[1])
}
