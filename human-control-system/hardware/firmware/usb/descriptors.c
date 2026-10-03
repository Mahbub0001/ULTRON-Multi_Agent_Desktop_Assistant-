#include "config.h"
#include "hid_reports.h"

static const uint8_t device_descriptor[] = {
    0x12,        // bLength
    0x01,        // bDescriptorType (Device)
    0x00, 0x02,  // bcdUSB 2.00
    0x00,        // bDeviceClass
    0x00,        // bDeviceSubClass
    0x00,        // bDeviceProtocol
    0x40,        // bMaxPacketSize0 (64)
    (uint8_t)(USB_VID & 0xFF), (uint8_t)(USB_VID >> 8),  // idVendor
    (uint8_t)(USB_PID & 0xFF), (uint8_t)(USB_PID >> 8),  // idProduct
    0x00, 0x01,  // bcdDevice 1.00
    0x01,        // iManufacturer
    0x02,        // iProduct
    0x03,        // iSerialNumber
    0x01,        // bNumConfigurations
};

static const uint8_t config_descriptor[] = {
    0x09,        // bLength
    0x02,        // bDescriptorType (Configuration)
    0x00, 0x00,  // wTotalLength (filled in at runtime)
    0x03,        // bNumInterfaces (Keyboard, Mouse, Raw HID)
    0x01,        // bConfigurationValue
    0x00,        // iConfiguration
    0x80,        // bmAttributes (Bus Powered)
    0x32,        // bMaxPower (100mA)

    // Interface 0: Keyboard
    0x09,        // bLength
    0x04,        // bDescriptorType (Interface)
    0x00,        // bInterfaceNumber
    0x00,        // bAlternateSetting
    0x01,        // bNumEndpoints
    0x03,        // bInterfaceClass (HID)
    0x01,        // bInterfaceSubClass (Boot)
    0x01,        // bInterfaceProtocol (Keyboard)
    0x00,        // iInterface

    // HID Descriptor (Keyboard)
    0x09,        // bLength
    0x21,        // bDescriptorType (HID)
    0x11, 0x01,  // bcdHID 1.11
    0x00,        // bCountryCode
    0x01,        // bNumDescriptors
    0x22,        // bDescriptorType (Report)
    0x00, 0x00,  // wDescriptorLength (filled at runtime)

    // Endpoint 1 IN (Keyboard)
    0x07,        // bLength
    0x05,        // bDescriptorType (Endpoint)
    0x81,        // bEndpointAddress (IN)
    0x03,        // bmAttributes (Interrupt)
    0x08, 0x00,  // wMaxPacketSize (8)
    0x01,        // bInterval (1ms)

    // Interface 1: Mouse
    0x09,        // bLength
    0x04,        // bDescriptorType (Interface)
    0x01,        // bInterfaceNumber
    0x00,        // bAlternateSetting
    0x01,        // bNumEndpoints
    0x03,        // bInterfaceClass (HID)
    0x00,        // bInterfaceSubClass
    0x02,        // bInterfaceProtocol (Mouse)
    0x00,        // iInterface

    // HID Descriptor (Mouse)
    0x09,        // bLength
    0x21,        // bDescriptorType (HID)
    0x11, 0x01,  // bcdHID 1.11
    0x00,        // bCountryCode
    0x01,        // bNumDescriptors
    0x22,        // bDescriptorType (Report)
    0x00, 0x00,  // wDescriptorLength (filled at runtime)

    // Endpoint 2 IN (Mouse)
    0x07,        // bLength
    0x05,        // bDescriptorType (Endpoint)
    0x82,        // bEndpointAddress (IN)
    0x03,        // bmAttributes (Interrupt)
    0x08, 0x00,  // wMaxPacketSize (8)
    0x01,        // bInterval (1ms)

    // Interface 2: Raw HID
    0x09,        // bLength
    0x04,        // bDescriptorType (Interface)
    0x02,        // bInterfaceNumber
    0x00,        // bAlternateSetting
    0x02,        // bNumEndpoints
    0x03,        // bInterfaceClass (HID)
    0x00,        // bInterfaceSubClass
    0x00,        // bInterfaceProtocol
    0x00,        // iInterface

    // HID Descriptor (Raw HID)
    0x09,        // bLength
    0x21,        // bDescriptorType (HID)
    0x11, 0x01,  // bcdHID 1.11
    0x00,        // bCountryCode
    0x01,        // bNumDescriptors
    0x22,        // bDescriptorType (Report)
    0x00, 0x00,  // wDescriptorLength (filled at runtime)

    // Endpoint 3 OUT (Raw HID RX)
    0x07,        // bLength
    0x05,        // bDescriptorType (Endpoint)
    0x03,        // bEndpointAddress (OUT)
    0x03,        // bmAttributes (Interrupt)
    0x40, 0x00,  // wMaxPacketSize (64)
    0x01,        // bInterval (1ms)

    // Endpoint 4 IN (Raw HID TX)
    0x07,        // bLength
    0x05,        // bDescriptorType (Endpoint)
    0x84,        // bEndpointAddress (IN)
    0x03,        // bmAttributes (Interrupt)
    0x40, 0x00,  // wMaxPacketSize (64)
    0x01,        // bInterval (1ms)
};

static const uint8_t keyboard_report_descriptor[] = {
    0x05, 0x01,        // Usage Page (Generic Desktop)
    0x09, 0x06,        // Usage (Keyboard)
    0xA1, 0x01,        // Collection (Application)
    0x85, HID_KEYBOARD_REPORT_ID,  // Report ID
    0x05, 0x07,        // Usage Page (Key Codes)
    0x19, 0xE0,        // Usage Minimum (224)
    0x29, 0xE7,        // Usage Maximum (231)
    0x15, 0x00,        // Logical Minimum (0)
    0x25, 0x01,        // Logical Maximum (1)
    0x75, 0x01,        // Report Size (1)
    0x95, 0x08,        // Report Count (8)
    0x81, 0x02,        // Input (Data, Variable, Absolute) - Modifiers
    0x95, 0x01,        // Report Count (1)
    0x75, 0x08,        // Report Size (8)
    0x81, 0x01,        // Input (Constant) - Reserved
    0x95, MAX_KEYS_PER_REPORT,  // Report Count (6)
    0x75, 0x08,        // Report Size (8)
    0x15, 0x00,        // Logical Minimum (0)
    0x25, 0x65,        // Logical Maximum (101)
    0x05, 0x07,        // Usage Page (Key Codes)
    0x19, 0x00,        // Usage Minimum (0)
    0x29, 0x65,        // Usage Maximum (101)
    0x81, 0x00,        // Input (Data, Array) - Keys
    0xC0,              // End Collection
};

static const uint8_t mouse_report_descriptor[] = {
    0x05, 0x01,        // Usage Page (Generic Desktop)
    0x09, 0x02,        // Usage (Mouse)
    0xA1, 0x01,        // Collection (Application)
    0x85, HID_MOUSE_REPORT_ID,  // Report ID
    0x09, 0x01,        // Usage (Pointer)
    0xA1, 0x00,        // Collection (Physical)
    0x05, 0x09,        // Usage Page (Button)
    0x19, 0x01,        // Usage Minimum (1)
    0x29, 0x05,        // Usage Maximum (5)
    0x15, 0x00,        // Logical Minimum (0)
    0x25, 0x01,        // Logical Maximum (1)
    0x95, 0x05,        // Report Count (5)
    0x75, 0x01,        // Report Size (1)
    0x81, 0x02,        // Input (Data, Variable, Absolute) - Buttons
    0x95, 0x01,        // Report Count (1)
    0x75, 0x03,        // Report Size (3)
    0x81, 0x01,        // Input (Constant) - Padding
    0x05, 0x01,        // Usage Page (Generic Desktop)
    0x09, 0x30,        // Usage (X)
    0x09, 0x31,        // Usage (Y)
    0x09, 0x38,        // Usage (Wheel)
    0x15, 0x81,        // Logical Minimum (-127)
    0x25, 0x7F,        // Logical Maximum (127)
    0x75, 0x08,        // Report Size (8)
    0x95, 0x03,        // Report Count (3)
    0x81, 0x06,        // Input (Data, Variable, Relative)
    0x05, 0x0C,        // Usage Page (Consumer)
    0x0A, 0x38, 0x02,  // Usage (AC Pan)
    0x15, 0x81,        // Logical Minimum (-127)
    0x25, 0x7F,        // Logical Maximum (127)
    0x75, 0x08,        // Report Size (8)
    0x95, 0x01,        // Report Count (1)
    0x81, 0x06,        // Input (Data, Variable, Relative)
    0xC0,              // End Collection (Physical)
    0xC0,              // End Collection (Application)
};

static const uint8_t consumer_report_descriptor[] = {
    0x05, 0x0C,        // Usage Page (Consumer)
    0x09, 0x01,        // Usage (Consumer Control)
    0xA1, 0x01,        // Collection (Application)
    0x85, HID_CONSUMER_REPORT_ID,  // Report ID
    0x15, 0x00,        // Logical Minimum (0)
    0x25, 0x01,        // Logical Maximum (1)
    0x09, 0xE9,        // Usage (Volume Up)
    0x09, 0xEA,        // Usage (Volume Down)
    0x09, 0xE2,        // Usage (Mute)
    0x09, 0xCD,        // Usage (Play/Pause)
    0x09, 0xB5,        // Usage (Next Track)
    0x09, 0xB6,        // Usage (Previous Track)
    0x09, 0xB7,        // Usage (Stop)
    0x75, 0x01,        // Report Size (1)
    0x95, 0x07,        // Report Count (7)
    0x81, 0x02,        // Input (Data, Variable, Absolute)
    0x95, 0x01,        // Report Count (1)
    0x75, 0x09,        // Report Size (9)
    0x81, 0x01,        // Input (Constant) - Padding
    0xC0,              // End Collection
};

static const uint8_t system_report_descriptor[] = {
    0x05, 0x01,        // Usage Page (Generic Desktop)
    0x09, 0x80,        // Usage (System Control)
    0xA1, 0x01,        // Collection (Application)
    0x85, HID_SYSTEM_REPORT_ID,  // Report ID
    0x15, 0x00,        // Logical Minimum (0)
    0x25, 0x01,        // Logical Maximum (1)
    0x09, 0x81,        // Usage (Power Down)
    0x09, 0x82,        // Usage (Sleep)
    0x09, 0x83,        // Usage (Wake Up)
    0x75, 0x01,        // Report Size (1)
    0x95, 0x03,        // Report Count (3)
    0x81, 0x02,        // Input (Data, Variable, Absolute)
    0x95, 0x01,        // Report Count (1)
    0x75, 0x05,        // Report Size (5)
    0x81, 0x01,        // Input (Constant) - Padding
    0xC0,              // End Collection
};

static const uint8_t raw_hid_report_descriptor[] = {
    0x06, 0x00, 0xFF,  // Usage Page (Vendor Defined 0xFF00)
    0x09, 0x01,        // Usage (Vendor Usage 1)
    0xA1, 0x01,        // Collection (Application)
    0x85, HID_RAW_REPORT_ID,  // Report ID
    0x15, 0x00,        // Logical Minimum (0)
    0x26, 0xFF, 0x00,  // Logical Maximum (255)
    0x75, 0x08,        // Report Size (8)
    0x95, RAW_HID_EPSIZE - 1,  // Report Count (63)
    0x09, 0x01,        // Usage (Vendor Usage 1)
    0x81, 0x02,        // Input (Data, Variable, Absolute)
    0x15, 0x00,        // Logical Minimum (0)
    0x26, 0xFF, 0x00,  // Logical Maximum (255)
    0x75, 0x08,        // Report Size (8)
    0x95, RAW_HID_EPSIZE - 1,  // Report Count (63)
    0x09, 0x01,        // Usage (Vendor Usage 1)
    0x91, 0x02,        // Output (Data, Variable, Absolute)
    0xC0,              // End Collection
};

static const uint8_t* const report_descriptors[] = {
    keyboard_report_descriptor,
    mouse_report_descriptor,
    consumer_report_descriptor,
    system_report_descriptor,
    raw_hid_report_descriptor,
};

static const uint16_t report_descriptor_sizes[] = {
    sizeof(keyboard_report_descriptor),
    sizeof(mouse_report_descriptor),
    sizeof(consumer_report_descriptor),
    sizeof(system_report_descriptor),
    sizeof(raw_hid_report_descriptor),
};

static const uint8_t string_descriptors[] = {
    // String 0: Language ID (English US)
    0x04, 0x03, 0x09, 0x04,

    // String 1: Manufacturer
    0x0E, 0x03,
    'M', 0x00, 'a', 0x00, 'r', 0x00, 'k', 0x00, '-', 0x00, 'L', 0x00, 'I', 0x00, 'V', 0x00,

    // String 2: Product
    0x1A, 0x03,
    'H', 0x00, 'u', 0x00, 'm', 0x00, 'a', 0x00, 'n', 0x00, ' ', 0x00,
    'C', 0x00, 'o', 0x00, 'n', 0x00, 't', 0x00, 'r', 0x00, 'o', 0x00, 'l', 0x00,
    ' ', 0x00, 'S', 0x00, 'y', 0x00, 's', 0x00, 't', 0x00, 'e', 0x00, 'm', 0x00,

    // String 3: Serial Number
    0x0E, 0x03,
    'H', 0x00, 'C', 0x00, 'S', 0x00, '0', 0x00, '0', 0x00, '1', 0x00,
};

const uint8_t* usb_get_device_descriptor(uint16_t* length) {
    *length = sizeof(device_descriptor);
    return device_descriptor;
}

const uint8_t* usb_get_config_descriptor(uint16_t* length) {
    *length = sizeof(config_descriptor);
    return config_descriptor;
}

const uint8_t* usb_get_string_descriptor(uint8_t index, uint16_t* length) {
    if (index >= sizeof(string_descriptors) / 2) {
        *length = 0;
        return NULL;
    }
    const uint8_t* ptr = string_descriptors;
    for (uint8_t i = 0; i <= index; i++) {
        if (i == index) {
            *length = ptr[0];
            return ptr;
        }
        ptr += ptr[0];
    }
    *length = 0;
    return NULL;
}

const uint8_t* usb_get_report_descriptor(uint8_t interface, uint16_t* length) {
    if (interface < 5) {
        *length = report_descriptor_sizes[interface];
        return report_descriptors[interface];
    }
    *length = 0;
    return NULL;
}

uint16_t usb_get_config_descriptor_total_length(void) {
    return sizeof(config_descriptor);
}