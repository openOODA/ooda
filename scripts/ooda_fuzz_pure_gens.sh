# sourced by ooda_fuzz_pure.sh — PRNG generators + domain emit helpers
# Domain-typed sample/call (uses DOMAIN, tmin, tmax, fname from caller)
# List[Int] element range constants (gen_list_int_val): min=-8, max=16

emit_fuzz_sample_let() {
  local ind="${1:-        }"
  case "$DOMAIN" in
    bool) echo "${ind}let __fuzz_x: Bool = gen_bool_val(__fuzz_prng_st);" ;;
    string) echo "${ind}let __fuzz_x: String = gen_string_val(__fuzz_prng_st, ${tmin}, ${tmax});" ;;
    list) echo "${ind}let __fuzz_x: List[Int] = gen_list_int_val(__fuzz_prng_st, ${tmin}, ${tmax});" ;;
    *) echo "${ind}let __fuzz_x: Int = gen_int_val(__fuzz_prng_st, ${tmin}, ${tmax});" ;;
  esac
}

emit_fuzz_call_let() {
  local ind="${1:-            }"
  case "$DOMAIN" in
    bool) echo "${ind}let __fuzz_r: Bool = ${fname}(__fuzz_x);" ;;
    string) echo "${ind}let __fuzz_r: String = ${fname}(__fuzz_x);" ;;
    list) echo "${ind}let __fuzz_r: List[Int] = ${fname}(__fuzz_x);" ;;
    *) echo "${ind}let __fuzz_r: Int = ${fname}(__fuzz_x);" ;;
  esac
}

emit_fuzz_generators() {
  cat <<'OO'
pub fn prng_step(__fuzz_st: Int) -> Int {
    let mut __fuzz_s = __fuzz_st;
    if __fuzz_s == 0 {
        __fuzz_s = 42;
    }
    let __fuzz_raw = __fuzz_s * 1103515245 + 12345;
    let __fuzz_next_st = __fuzz_raw - (__fuzz_raw / 2147483648) * 2147483648;
    if __fuzz_next_st < 0 {
        return 0 - __fuzz_next_st;
    }
    return __fuzz_next_st;
}

pub fn gen_int_val(__fuzz_st: Int, __fuzz_min_v: Int, __fuzz_max_v: Int) -> Int {
    let __fuzz_abs_raw = prng_step(__fuzz_st);
    let __fuzz_span = __fuzz_max_v - __fuzz_min_v + 1;
    if __fuzz_span <= 0 {
        return __fuzz_min_v;
    }
    let __fuzz_rem = __fuzz_abs_raw - (__fuzz_abs_raw / __fuzz_span) * __fuzz_span;
    return __fuzz_min_v + __fuzz_rem;
}

pub fn gen_bool_val(__fuzz_st: Int) -> Bool {
    let __fuzz_abs_raw = prng_step(__fuzz_st);
    let __fuzz_bit = __fuzz_abs_raw - (__fuzz_abs_raw / 2) * 2;
    if __fuzz_bit != 0 {
        return true;
    }
    return false;
}

pub fn gen_string_val(__fuzz_st: Int, __fuzz_min_len: Int, __fuzz_max_len: Int) -> String {
    let mut __fuzz_lo = __fuzz_min_len;
    let mut __fuzz_hi = __fuzz_max_len;
    if __fuzz_hi < __fuzz_lo {
        __fuzz_hi = __fuzz_lo;
    }
    let __fuzz_abs_raw = prng_step(__fuzz_st);
    let __fuzz_span = __fuzz_hi - __fuzz_lo + 1;
    let __fuzz_rem = __fuzz_abs_raw - (__fuzz_abs_raw / __fuzz_span) * __fuzz_span;
    let __fuzz_len = __fuzz_lo + __fuzz_rem;
    let __fuzz_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let __fuzz_num_chars = chars_len(__fuzz_chars);
    let mut __fuzz_out = "";
    let mut __fuzz_i = 0;
    let mut __fuzz_cur = __fuzz_abs_raw;
    while __fuzz_i < __fuzz_len {
        __fuzz_cur = prng_step(__fuzz_cur);
        let __fuzz_idx = __fuzz_cur - (__fuzz_cur / __fuzz_num_chars) * __fuzz_num_chars;
        __fuzz_out = __fuzz_out + char_at(__fuzz_chars, __fuzz_idx);
        __fuzz_i = __fuzz_i + 1;
    }
    return __fuzz_out;
}

// List[Int]: length in [min_len,max_len]; elements in [-8,16] (fixed)
pub fn gen_list_int_val(__fuzz_st: Int, __fuzz_min_len: Int, __fuzz_max_len: Int) -> List[Int] {
    let mut __fuzz_lo = __fuzz_min_len;
    let mut __fuzz_hi = __fuzz_max_len;
    if __fuzz_hi < __fuzz_lo {
        __fuzz_hi = __fuzz_lo;
    }
    let __fuzz_abs_raw = prng_step(__fuzz_st);
    let __fuzz_span = __fuzz_hi - __fuzz_lo + 1;
    let __fuzz_rem = __fuzz_abs_raw - (__fuzz_abs_raw / __fuzz_span) * __fuzz_span;
    let __fuzz_len = __fuzz_lo + __fuzz_rem;
    let __fuzz_elem_min = 0 - 8;
    let __fuzz_elem_max = 16;
    let mut __fuzz_out: List[Int] = list_new();
    let mut __fuzz_i = 0;
    let mut __fuzz_cur = __fuzz_abs_raw;
    while __fuzz_i < __fuzz_len {
        __fuzz_cur = prng_step(__fuzz_cur);
        let __fuzz_elem = gen_int_val(__fuzz_cur, __fuzz_elem_min, __fuzz_elem_max);
        __fuzz_out = list_push(__fuzz_out, __fuzz_elem);
        __fuzz_i = __fuzz_i + 1;
    }
    return __fuzz_out;
}

// Backend-C cannot lower OoIList == OoIList; pure deep-eq for list ensures.
pub fn list_eq_int(__fuzz_a: List[Int], __fuzz_b: List[Int]) -> Bool {
    if list_len(__fuzz_a) != list_len(__fuzz_b) {
        return false;
    }
    let __fuzz_n = list_len(__fuzz_a);
    let mut __fuzz_i = 0;
    while __fuzz_i < __fuzz_n {
        if list_get(__fuzz_a, __fuzz_i) != list_get(__fuzz_b, __fuzz_i) {
            return false;
        }
        __fuzz_i = __fuzz_i + 1;
    }
    return true;
}
OO
}
