#!/bin/bash
sed -i 's/if chars_len(src) > 200000 {/if chars_len(src) > 20000000 {/' oodac/main.oo
bin/ooda build oodac/main.oo bin/ooda2
time bin/ooda2 check oodac/main.oo
