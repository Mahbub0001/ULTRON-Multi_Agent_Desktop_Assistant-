#include "system.h"
#include "hid_reports.h"
#include "config.h"
#include "util.h"

static uint8_t system_usage = 0;

void system_init(void) {
    system_usage = 0;
}

void system_press(uint8_t usage) {
    system_usage = usage;
    system_send_report();
}

void system_release(uint8_t usage) {
    (void)usage;
    system_usage = 0;
    system_send_report();
}

void system_tap(uint8_t usage) {
    system_press(usage);
    delay_ms(10);
    system_release(usage);
}

void system_send_report(void) {
    extern bool hid_send_system_report(const hid_system_report_t* report);

    hid_system_report_t report = {
        .report_id = HID_SYSTEM_REPORT_ID,
        .usage = system_usage
    };

    hid_send_system_report(&report);
}

bool system_is_pressed(uint8_t usage) {
    return system_usage == usage;
}