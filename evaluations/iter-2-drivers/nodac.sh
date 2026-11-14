#!/bin/bash
# run "$@" as uid 0 without CAP_DAC_OVERRIDE / CAP_DAC_READ_SEARCH, so owner permission bits apply
exec capsh --drop=cap_dac_override,cap_dac_read_search -- -c 'exec "$@"' nodac "$@"
