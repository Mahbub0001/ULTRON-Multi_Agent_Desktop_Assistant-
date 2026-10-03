#include <stdint.h>
#include <stdbool.h>
#include "config.h"
#include "hid_reports.h"
#include "protocol.h"
#include "input/keyboard.h"
#include "input/mouse.h"
#include "input/consumer.h"
#include "input/system.h"
#include "usb/descriptors.h"
#include "usb/callbacks.h"
#include "usb/core.h"
#include "protocol/ack.h"
#include "util.h"

static uint32_t systick_counter = 0;

void SysTick_Handler(void) {
    systick_counter++;
}

uint32_t get_uptime_ms(void) {
    return systick_counter;
}

uint32_t usb_get_uptime_ms(void) {
    return systick_counter;
}

void delay_ms(uint16_t ms) {
    uint32_t start = systick_counter;
    while (systick_counter - start < ms) {
        __asm volatile ("nop");
    }
}

void board_init(void) {
    SysTick_Config(CPU_FREQUENCY_MHZ * 1000);
    
    #if LED_ACTIVE_HIGH
    #else
    #endif
}



int main(void) {
    board_init();
    
    keyboard_init();
    mouse_init();
    consumer_init();
    system_init();
    
    protocol_init();
    ack_init();
    protocol_handlers_init();
    
    usb_callbacks_init();
    usb_init();
    
    while (1) {
        usb_task();
        
        usb_callback_tick();
        
        ack_tick(usb_get_uptime_ms());
        
        keyboard_send_report();
        mouse_send_report();
        consumer_send_report();
        system_send_report();
    }
    
    return 0;
}

void HardFault_Handler(void) {
    while (1) {
        __asm volatile ("bkpt #0");
    }
}

void MemManage_Handler(void) {
    while (1) {
        __asm volatile ("bkpt #0");
    }
}

void BusFault_Handler(void) {
    while (1) {
        __asm volatile ("bkpt #0");
    }
}

void UsageFault_Handler(void) {
    while (1) {
        __asm volatile ("bkpt #0");
    }
}