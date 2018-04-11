.text
.align 2
.global _start

.set DISPCNT, 0x04000000
.set MODE_3, 3
.set BG2, 0x400

.set VRAM, 0x06000000
.set WIDTH, 240
.set HEIGHT, 160
.set NUM_PIXELS, WIDTH * HEIGHT

.set RED, 0x1F
.set GREEN, 0x03E0
.set BLUE, 0x7C00
.set YELLOW, RED | GREEN

.set COLOR_KEY, 0x7C1F

.set TIMER0_VALUE, 0x04000100
.set TIMER0_CONTROL, 0x04000102
.set TIMER1_VALUE, 0x04000104
.set TIMER1_CONTROL, 0x04000106
.set TIMER2_VALUE, 0x04000108
.set TIMER2_CONTROL, 0x0400010A
.set TIMER3_VALUE, 0x0400010C
.set TIMER3_CONTROL, 0x0400010E

.set TM0_START, 48756

.set DMA3_SRC, 0x040000D4
.set DMA3_DST, 0x040000D8
.set DMA3_COUNT, 0x040000DC
.set DMA3_CONTROL, 0x040000DE

.set EWRAM, 0x02000000
.set IWRAM, 0x03000000

@ .set CODE_LENGTH, end - main

_start:
	@ init display to mode 3 with background 2 enabled
	mov r0, #DISPCNT
	mov r1, #MODE_3
	orr r1, #BG2
	strh r1, [r0]

main:
	mov r0, #RED
	@bl fill_screen

	mov r0, #5
	mov r1, #5
	mov r2, #20
	mov r3, #BLUE
	bl draw_line

	main_loop:
		b main_loop

@ r0: x-coordinate
@ r1: y-coordinate
@ r2: color
draw_pixel:
	stmfd r13!, {r0-r4, r14}

	mov r3, #VRAM
	mov r4, #WIDTH

	@ multiply y by width, and add x
	mla r0, r4, r1, r0

	@ add offset in R0 to VRAM after multiplying it by 2 (2 bytes per pixel)
	add r3, r3, r0, lsl #1

	strh r2, [r3]

	ldmfd r13!, {r0-r4, r14}
	mov r15, r14

@ r0: x-coordinate
@ r1: y-coordinate
@ r2: length
@ r3: color
draw_line:
	line_loop:
		stmfd r13!, {r0-r4, r14}

		mov r2, r3
		bl draw_pixel

		ldmfd r13!, {r0-r4, r14}

		subs r2, #1
		add r0, #1
		bne line_loop

	mov r15, r14

@ r0: color
fill_screen:
	mov r2, r0
	mov r0, #0
	mov r1, #0

	height_loop:
		width_loop:
			stmfd r13!, {r0-r4, r14}
			bl draw_pixel
			ldmfd r13!, {r0-r4, r14}

			add r0, #1
			cmp r0, #WIDTH
			bne width_loop

		add r1, #1
		cmp r1, #HEIGHT
		bne height_loop

	mov r15, r14
