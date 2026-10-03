#include "core.h"
#include "descriptors.h"
#include "callbacks.h"
#include "config.h"

#define USB_BASE_ADDR 0x402E0000

#define USB_USBCMD        (*(volatile uint32_t*)(USB_BASE_ADDR + 0x140))
#define USB_USBSTS        (*(volatile uint32_t*)(USB_BASE_ADDR + 0x144))
#define USB_USBINTR       (*(volatile uint32_t*)(USB_BASE_ADDR + 0x148))
#define USB_FRINDEX       (*(volatile uint32_t*)(USB_BASE_ADDR + 0x14C))
#define USB_DEVICEADDR    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x154))
#define USB_ENDPTLISTADDR (*(volatile uint32_t*)(USB_BASE_ADDR + 0x158))
#define USB_BURSTSIZE     (*(volatile uint32_t*)(USB_BASE_ADDR + 0x160))
#define USB_TXFILLTUNING  (*(volatile uint32_t*)(USB_BASE_ADDR + 0x164))
#define USB_ENDPTNAK      (*(volatile uint32_t*)(USB_BASE_ADDR + 0x178))
#define USB_ENDPTNAKEN    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x17C))
#define USB_PORTSC1       (*(volatile uint32_t*)(USB_BASE_ADDR + 0x184))
#define USB_OTGSC         (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1A4))
#define USB_USBMODE       (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1A8))
#define USB_ENDPTSETUPSTAT (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1AC))
#define USB_ENDPTPRIME    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1B0))
#define USB_ENDPTFLUSH    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1B4))
#define USB_ENDPTSTATUS   (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1B8))
#define USB_ENDPTCOMPLETE (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1BC))
#define USB_ENDPTCTRL0    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1C0))
#define USB_ENDPTCTRL1    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1C4))
#define USB_ENDPTCTRL2    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1C8))
#define USB_ENDPTCTRL3    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1CC))
#define USB_ENDPTCTRL4    (*(volatile uint32_t*)(USB_BASE_ADDR + 0x1D0))

#define USBCMD_RS         (1 << 0)
#define USBCMD_RST        (1 << 1)

#define USBSTS_UI         (1 << 0)
#define USBSTS_UEI        (1 << 1)
#define USBSTS_PCI        (1 << 2)
#define USBSTS_URI        (1 << 6)
#define USBSTS_SRI        (1 << 7)
#define USBSTS_SLI        (1 << 8)
#define USBSTS_NAKI       (1 << 16)

#define USBINTR_UE        (1 << 0)
#define USBINTR_UEE       (1 << 1)
#define USBINTR_PCE       (1 << 2)
#define USBINTR_URE       (1 << 6)
#define USBINTR_SRE       (1 << 7)
#define USBINTR_SLE       (1 << 8)
#define USBINTR_NAKE      (1 << 16)

#define PORTSC_CCS        (1 << 0)
#define PORTSC_PEC        (1 << 1)
#define PORTSC_PRC        (1 << 4)
#define PORTSC_OCC        (1 << 5)
#define PORTSC_OCA        (1 << 6)
#define PORTSC_PEC_MASK   (3 << 1)
#define PORTSC_SPEED_MASK (3 << 26)

#define USBMODE_CM_DEVICE (2 << 0)
#define USBMODE_SLOM      (1 << 3)
#define USBMODE_SDIS      (1 << 4)

#define ENDPTCTRL_RX_ENABLE   (1 << 0)
#define ENDPTCTRL_RX_TYPE_INT (3 << 18)
#define ENDPTCTRL_TX_ENABLE   (1 << 16)
#define ENDPTCTRL_TX_TYPE_INT (3 << 2)

typedef struct {
    uint32_t next;
    uint32_t config;
    uint32_t buffer;
    uint32_t reserved[4];
} __attribute__((aligned(32))) qtd_t;

typedef struct {
    uint32_t config;
    qtd_t* qtd_head;
    qtd_t* qtd_current;
    qtd_t* qtd_next;
    uint32_t reserved[4];
} __attribute__((aligned(32))) qh_t;

static qh_t endpoint_qh[5] __attribute__((aligned(2048)));
static qtd_t endpoint_qtd[5][4] __attribute__((aligned(32)));
static uint8_t endpoint_buffer[5][64] __attribute__((aligned(64)));

static volatile uint8_t usb_device_state = 0;
static volatile uint8_t usb_address = 0;
static volatile uint8_t usb_configuration = 0;

void usb_init(void) {
    USB_USBCMD = USBCMD_RST;
    while (USB_USBCMD & USBCMD_RST);

    USB_USBMODE = USBMODE_CM_DEVICE | USBMODE_SLOM | USBMODE_SDIS;

    for (int i = 0; i < 5; i++) {
        endpoint_qh[i].config = 0;
        endpoint_qh[i].qtd_head = &endpoint_qtd[i][0];
        endpoint_qh[i].qtd_current = 0;
        endpoint_qh[i].qtd_next = 1;
    }

    USB_ENDPTLISTADDR = (uint32_t)endpoint_qh;
    USB_USBINTR = USBINTR_UE | USBINTR_UEE | USBINTR_PCE | USBINTR_URE | USBINTR_SRE | USBINTR_SLE | USBINTR_NAKE;
    USB_USBCMD = USBCMD_RS;

    usb_callbacks_init();
    usb_device_state = 1;
}

void usb_task(void) {
    uint32_t status = USB_USBSTS;
    USB_USBSTS = status;

    if (status & USBSTS_UI) {
        usb_isr();
    }

    if (status & USBSTS_URI) {
        usb_callback_reset();
    }

    if (status & USBSTS_PCI) {
        uint32_t setup_stat = USB_ENDPTSETUPSTAT;
        if (setup_stat & 1) {
            USB_ENDPTSETUPSTAT = 1;
            usb_handle_setup();
        }
    }

    if (status & USBSTS_NAKI) {
        // Handle NAK
    }
}

void usb_isr(void) {
    usb_task();
}

void usb_handle_setup(void) {
    static uint8_t setup_buffer[8];
    qtd_t* qtd = endpoint_qh[0].qtd_head;
    
    if (qtd->config & (1 << 7)) {
        return;
    }

    volatile uint32_t* setup_ptr = (volatile uint32_t*)qtd->buffer;
    setup_buffer[0] = setup_ptr[0] & 0xFF;
    setup_buffer[1] = (setup_ptr[0] >> 8) & 0xFF;
    setup_buffer[2] = (setup_ptr[0] >> 16) & 0xFF;
    setup_buffer[3] = (setup_ptr[0] >> 24) & 0xFF;
    setup_buffer[4] = setup_ptr[1] & 0xFF;
    setup_buffer[5] = (setup_ptr[1] >> 8) & 0xFF;
    setup_buffer[6] = (setup_ptr[1] >> 16) & 0xFF;
    setup_buffer[7] = (setup_ptr[1] >> 24) & 0xFF;

    uint8_t bmRequestType = setup_buffer[0];
    uint8_t bRequest = setup_buffer[1];
    uint16_t wValue = setup_buffer[2] | (setup_buffer[3] << 8);
    uint16_t wIndex = setup_buffer[4] | (setup_buffer[5] << 8);
    uint16_t wLength = setup_buffer[6] | (setup_buffer[7] << 8);

    uint8_t response[64] = {0};
    uint16_t response_len = 0;
    bool handled = false;

    if ((bmRequestType & 0x60) == 0x00) {
        handled = usb_handle_standard_request(bmRequestType, bRequest, wValue, wIndex, wLength, response, &response_len);
    } else if ((bmRequestType & 0x60) == 0x20) {
        handled = usb_callback_control_request(bRequest, bmRequestType, wValue, wIndex, wLength, response);
        response_len = wLength;
    }

    if (handled) {
        usb_send_callback(0, response, response_len);
    } else {
        USB_ENDPTCTRL0 |= (1 << 16) | (1 << 0);
    }

    qtd->config &= ~(1 << 7);
    USB_ENDPTPRIME = 1;
}

bool usb_handle_standard_request(uint8_t bmRequestType, uint8_t bRequest, uint16_t wValue, uint16_t wIndex, uint16_t wLength, uint8_t* response, uint16_t* response_len) {
    uint16_t length = 0;
    const uint8_t* data = NULL;

    switch (bRequest) {
        case 0x05: // SET_ADDRESS
            usb_address = wValue & 0x7F;
            USB_DEVICEADDR = (usb_address << 25) | (1 << 24);
            *response_len = 0;
            return true;

        case 0x06: // GET_DESCRIPTOR
            {
                uint8_t desc_type = (wValue >> 8) & 0xFF;
                uint8_t desc_index = wValue & 0xFF;

                switch (desc_type) {
                    case 0x01: // Device
                        data = usb_get_device_descriptor(&length);
                        break;
                    case 0x02: // Configuration
                        data = usb_get_config_descriptor(&length);
                        break;
                    case 0x03: // String
                        data = usb_get_string_descriptor(desc_index, &length);
                        break;
                    case 0x22: // Report
                        data = usb_get_report_descriptor(wIndex & 0xFF, &length);
                        break;
                }

                if (data && length > 0) {
                    if (length > wLength) length = wLength;
                    memcpy(response, data, length);
                    *response_len = length;
                    return true;
                }
            }
            break;

        case 0x09: // SET_CONFIGURATION
            usb_configuration = wValue & 0xFF;
            usb_callback_set_configuration(usb_configuration);
            *response_len = 0;
            return true;

        case 0x01: // GET_STATUS
            response[0] = 0x00;
            response[1] = 0x00;
            *response_len = 2;
            return true;

        case 0x0A: // SET_INTERFACE
            *response_len = 0;
            return true;

        case 0x0B: // SET_FEATURE
        case 0x0C: // CLEAR_FEATURE
            *response_len = 0;
            return true;
    }

    return false;
}

bool usb_configured(void) {
    return usb_configuration != 0;
}

uint8_t usb_get_state(void) {
    return usb_device_state;
}

void usb_send_callback(uint8_t endpoint, const uint8_t* data, uint16_t length) {
    if (endpoint >= 5) return;

    qh_t* qh = &endpoint_qh[endpoint];
    qtd_t* qtd = qh->qtd_head;

    qtd->config = (length << 16) | (1 << 7) | (1 << 15);
    qtd->buffer = (uint32_t)data;
    qtd->next = 1;

    qh->qtd_current = qtd;
    qh->qtd_next = 1;

    USB_ENDPTPRIME = (1 << (endpoint * 2));
}

bool usb_endpoint_ready(uint8_t endpoint) {
    if (endpoint >= 5) return false;
    return !(USB_ENDPTSTATUS & (1 << (endpoint * 2)));
}