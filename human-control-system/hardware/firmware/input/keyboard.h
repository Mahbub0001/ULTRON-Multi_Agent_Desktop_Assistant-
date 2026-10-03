#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "config.h"
#include "hid_reports.h"

void keyboard_init(void);
void keyboard_press(uint16_t usage_page, uint16_t usage, uint8_t modifiers);
void keyboard_release(uint16_t usage_page, uint16_t usage);
void keyboard_tap(uint16_t usage_page, uint16_t usage, uint16_t delay_ms);
void keyboard_set_modifiers(uint8_t modifiers);
void keyboard_clear_modifiers(void);
void keyboard_release_all(void);
void keyboard_send_report(void);

uint8_t keyboard_get_modifiers(void);
uint8_t keyboard_get_num_pressed(void);
const uint8_t* keyboard_get_pressed_keys(void);
bool keyboard_is_key_pressed(uint16_t usage);