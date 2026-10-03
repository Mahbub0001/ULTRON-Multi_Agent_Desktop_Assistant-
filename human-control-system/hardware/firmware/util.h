#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

void delay_ms(uint16_t ms);
uint32_t get_uptime_ms(void);

#ifdef __cplusplus
}
#endif