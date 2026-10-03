#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "config.h"
#include "hid_reports.h"

void mouse_init(void);
void mouse_move(int16_t x, int16_t y, bool absolute);
void mouse_press(uint8_t button);
void mouse_release(uint8_t button);
void mouse_click(uint8_t button);
void mouse_wheel(int8_t wheel, int8_t pan);
void mouse_send_report(void);

void mouse_get_position(int16_t* x, int16_t* y);
uint8_t mouse_get_buttons(void);
bool mouse_is_button_pressed(uint8_t button);
void mouse_set_absolute_mode(bool absolute);
bool mouse_is_absolute_mode(void);