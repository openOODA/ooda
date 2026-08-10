# Detect parameter count of target fn from body file (single-line signature).
fuzz_fn_arity() {
  local fname="$1" body="$2" line inside commas
  line=$(grep -E "^(pub[[:space:]]+)?fn[[:space:]]+${fname}[[:space:]]*\\(" "$body" | head -1 || true)
  [[ -z "$line" ]] && { echo "ERR	fuzz	no signature for target '${fname}'" >&2; return 2; }
  inside="${line#*(}"; inside="${inside%%)*}"
  inside="${inside#"${inside%%[![:space:]]*}"}"
  inside="${inside%"${inside##*[![:space:]]}"}"
  [[ -z "$inside" ]] && { echo 0; return 0; }
  commas="${inside//[^,]/}"; echo $((${#commas} + 1))
}

# True if every param of fname is typed Int (multi-arg Int gate).
fuzz_fn_params_all_int() {
  local fname="$1" body="$2" line inside p
  line=$(grep -E "^(pub[[:space:]]+)?fn[[:space:]]+${fname}[[:space:]]*\\(" "$body" | head -1 || true)
  [[ -z "$line" ]] && return 1
  inside="${line#*(}"; inside="${inside%%)*}"
  IFS=',' read -ra parts <<<"$inside"
  for p in "${parts[@]}"; do
    p="${p#"${p%%[![:space:]]*}"}"; p="${p%"${p##*[![:space:]]}"}"
    [[ "$p" =~ :[[:space:]]*Int$ ]] || return 1
  done
  return 0
}

# True if every param is typed Bool (M56 multi-arg Bool gate).
fuzz_fn_params_all_bool() {
  local fname="$1" body="$2" line inside p
  line=$(grep -E "^(pub[[:space:]]+)?fn[[:space:]]+${fname}[[:space:]]*\\(" "$body" | head -1 || true)
  [[ -z "$line" ]] && return 1
  inside="${line#*(}"; inside="${inside%%)*}"
  IFS=',' read -ra parts <<<"$inside"
  for p in "${parts[@]}"; do
    p="${p#"${p%%[![:space:]]*}"}"; p="${p%"${p##*[![:space:]]}"}"
    [[ "$p" =~ :[[:space:]]*Bool$ ]] || return 1
  done
  return 0
}

# True if every param is typed String (M106 multi-arg String gate).
fuzz_fn_params_all_string() {
  local fname="$1" body="$2" line inside p
  line=$(grep -E "^(pub[[:space:]]+)?fn[[:space:]]+${fname}[[:space:]]*\\(" "$body" | head -1 || true)
  [[ -z "$line" ]] && return 1
  inside="${line#*(}"; inside="${inside%%)*}"
  IFS=',' read -ra parts <<<"$inside"
  for p in "${parts[@]}"; do
    p="${p#"${p%%[![:space:]]*}"}"; p="${p%"${p##*[![:space:]]}"}"
    [[ "$p" =~ :[[:space:]]*String$ ]] || return 1
  done
  return 0
}

# Fail-closed: arity 0; arity≥4; multi List; wrong multi param types.
# In: Int arity-2/3; Bool/String arity-2.
fuzz_check_arity() {
  local fname="$1" arity="$2" body="$3"
  if [[ "$arity" -le 0 ]]; then
    echo "ERR	fuzz	target '${fname}' has no parameters (fail-closed)" >&2; return 2
  fi
  if [[ "$arity" -ge 4 ]]; then
    echo "ERR	fuzz	arity>=4 fail-closed for pure path (target ${fname} arity=${arity})" >&2; return 2
  fi
  if [[ "$arity" -ge 2 ]]; then
    if [[ "$DOMAIN" == "int" ]]; then
      if ! fuzz_fn_params_all_int "$fname" "$body"; then
        echo "ERR	fuzz	arity-${arity} pure path requires all Int params (target ${fname})" >&2; return 2
      fi
    elif [[ "$DOMAIN" == "bool" ]]; then
      if [[ "$arity" -ge 3 ]]; then
        echo "ERR	fuzz	bool multi-arg arity>=3 fail-closed (target ${fname})" >&2; return 2
      fi
      if ! fuzz_fn_params_all_bool "$fname" "$body"; then
        echo "ERR	fuzz	arity-2 pure bool path requires all Bool params (target ${fname})" >&2; return 2
      fi
    elif [[ "$DOMAIN" == "string" ]]; then
      if [[ "$arity" -ge 3 ]]; then
        echo "ERR	fuzz	string multi-arg arity>=3 fail-closed (target ${fname})" >&2; return 2
      fi
      if ! fuzz_fn_params_all_string "$fname" "$body"; then
        echo "ERR	fuzz	arity-2 pure string path requires all String params (target ${fname})" >&2; return 2
      fi
    else
      echo "ERR	fuzz	multi-arg non-int fail-closed (domain=${DOMAIN} target=${fname})" >&2; return 2
    fi
  fi
  return 0
}

# Rewrite marker expr: x/y/z/result → harness locals (arity-aware).
fuzz_rewrite_expr() {
  local expr="$1" kind="$2" a="${ARITY:-1}" sedp='s/\bx\b/__fuzz_x/g'
  [[ "$a" -ge 2 ]] && sedp="s/\by\b/__fuzz_y/g; $sedp"
  [[ "$a" -ge 3 ]] && sedp="s/\bz\b/__fuzz_z/g; $sedp"
  [[ "$kind" == "ens" ]] && sedp="s/\bresult\b/__fuzz_r/g; $sedp"
  echo "$expr" | sed "$sedp"
}
