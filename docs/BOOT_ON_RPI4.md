# Boot AIOS Kernel on Real Raspberry Pi 4

Step-by-step guide to run the kernel on physical RPi 4 hardware.

## Prerequisites

- Raspberry Pi 4 (or 3)
- MicroSD card (8GB+)
- USB–TTL adapter (3.3V; e.g. CP2102, FT232)
- macOS or Linux host for SD preparation

## Step 1: Build the kernel

```bash
./tools/build-rpi.sh
```

Output: `target/aarch64-unknown-none/release/kernel8.img`

**Do not** use `--raspi3` — that is for raspi3 only. RPi 4 uses the default build.

## Step 2: Prepare the SD card

### Option A: Raspberry Pi Imager (easiest)

1. Download [Raspberry Pi Imager](https://www.raspberrypi.com/software/)
2. Flash **Raspberry Pi OS Lite (64-bit)** to the SD card
3. Eject, re-insert, mount the **boot** partition (first partition, FAT32)
4. On the boot partition:
   - Add `enable_uart=1` to `config.txt` (at the end or in `[all]` section)
   - Replace `kernel8.img` with our built file:
     ```bash
     cp target/aarch64-unknown-none/release/kernel8.img /Volumes/bootfs/kernel8.img
     # or on Linux: cp ... /media/$USER/bootfs/kernel8.img
     ```
   - Optionally add `SKILL.md` to the root for `load`:
     ```bash
     cp SKILL.md /Volumes/bootfs/
     ```
5. Unmount and eject

### Option B: Manual boot files

If you have existing RPi boot files (start4.elf, fixup4.dat, config.txt):

1. Copy them to the SD boot partition
2. Add to `config.txt`:
   ```
   enable_uart=1
   kernel=kernel8.img
   ```
3. Copy our `kernel8.img` to the boot partition root
4. Add `SKILL.md` (optional)

## Step 3: Connect UART

| RPi 4 Pin | Adapter   |
|----------|-----------|
| GPIO14 (pin 8) — TX | RX on USB–TTL |
| GPIO15 (pin 10) — RX | TX on USB–TTL |
| GND (pin 6) | GND |

Use **3.3V** level USB–TTL (not 5V).

## Step 4: Boot and connect serial

1. Insert SD, connect power
2. Connect USB–TTL to host PC
3. Open serial terminal at **115200 baud**:
   - macOS: `screen /dev/tty.usbserial-* 115200` or `cu -l /dev/tty.usbserial-* -s 115200`
   - Linux: `screen /dev/ttyUSB0 115200` or `minicom -D /dev/ttyUSB0 -b 115200`

## Step 5: Verify

You should see:

```
    ___    ________  _____
   /   |  /  _/ __ \/ ___/
  ...
>> AIOS kernel ready. help, time, load, skills, mem, sd, uptime, ...
>> 
```

Try `sd` — on real RPi 4 it should report SD OK (EMMC2 at 0xFE340000). Try `load` to read SKILL.md from SD.

## Troubleshooting

| Symptom | Check |
|---------|-------|
| No output | enable_uart=1 in config.txt; correct UART pins; 115200 baud |
| Black screen only | kernel8.img copied; ARM 64-bit build |
| SD init timeout | Ensure SKILL.md is on first FAT32 partition root |
