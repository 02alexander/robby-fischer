MEMORY {
    BOOT2 : org = 0x10000000, len = 0x00000100
    FLASH : org = 0x10000100, len = 0x001FFF00
    RAM   : org = 0x20000000, len = 0x00040000
}

EXTERN(BOOT2_FIRMWARE)

SECTIONS {
    .boot2 ORIGIN(BOOT2) : {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;
