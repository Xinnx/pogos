section .multiboot_header
header_start:
    dd 0xe85250d6   ;magic number
    dd 0            ; arch 0 (protected i386)
    dd header_end - header_start
    ; checksum
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))

    ; multiboot tags

    ; end
    dw 0
    dw 0
    dd 8
header_end:
