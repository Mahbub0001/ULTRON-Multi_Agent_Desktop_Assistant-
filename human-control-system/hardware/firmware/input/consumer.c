#include "consumer.h"
#include "hid_reports.h"
#include "config.h"
#include "util.h"

static uint16_t consumer_usage = 0;

void consumer_init(void) {
    consumer_usage = 0;
}

void consumer_press(uint16_t usage) {
    consumer_usage = usage;
    consumer_send_report();
}

void consumer_release(uint16_t usage) {
    (void)usage;
    consumer_usage = 0;
    consumer_send_report();
}

void consumer_tap(uint16_t usage) {
    consumer_press(usage);
    delay_ms(10);
    consumer_release(usage);
}

void consumer_send_report(void) {
    extern bool hid_send_consumer_report(const hid_consumer_report_t* report);

    hid_consumer_report_t report = {
        .report_id = HID_CONSUMER_REPORT_ID,
        .usage = consumer_usage
    };

    hid_send_consumer_report(&report);
}

bool consumer_is_pressed(uint16_t usage) {
    return consumer_usage == usage;
}