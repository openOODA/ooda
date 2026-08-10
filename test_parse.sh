#!/bin/sh
OODAC_BIN=bootstrap/seed/oodac ./scripts/oodac_pure_build.sh oodac/main.oo oodac/oodac
./oodac/oodac run fixtures/println_type_smoke.oo
