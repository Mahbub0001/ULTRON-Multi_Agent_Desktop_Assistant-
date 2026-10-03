#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "config.h"
#include "hid_reports.h"

void consumer_init(void);
void consumer_press(uint16_t usage);
void consumer_release(uint16_t usage);
void consumer_tap(uint16_t usage);
void consumer_send_report(void);

bool consumer_is_pressed(uint16_t usage);