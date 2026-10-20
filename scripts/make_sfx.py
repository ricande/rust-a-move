#!/usr/bin/env python3
"""Synthesizes the game's sound effects into assets/sfx/ (16-bit PCM mono,
22 050 Hz). Deterministic: running it again gives the same files. The
sounds are original and covered by the project's license.

    python3 scripts/make_sfx.py
"""
import math
import os
import random
import struct
import wave

RATE = 22050
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "sfx")


def env(i, n, attack=0.004, power=2.0):
    a = int(attack * RATE)
    if i < a:
        return i / max(a, 1)
    return (1 - (i - a) / max(n - a, 1)) ** power


def tone(f0, f1, secs, vol=0.5, shape="sine", power=2.0, attack=0.004):
    n = int(secs * RATE)
    out, phase = [], 0.0
    for i in range(n):
        t = i / n
        f = f0 * (f1 / f0) ** t
        phase += f / RATE
        p = phase % 1.0
        if shape == "sine":
            s = math.sin(2 * math.pi * p)
        elif shape == "tri":
            s = 1 - 4 * abs(p - 0.5)
        else:
            s = 1.0 if p < 0.5 else -1.0
        out.append(s * env(i, n, attack, power) * vol)
    return out


def noise(secs, vol=0.4, power=2.0, seed=1, smooth=0.0):
    rng = random.Random(seed)
    n = int(secs * RATE)
    out, last = [], 0.0
    for i in range(n):
        x = rng.uniform(-1, 1)
        last = last * smooth + x * (1 - smooth)
        out.append(last * env(i, n, 0.002, power) * vol)
    return out


def mix(*parts):
    n = max(len(p) for p in parts)
    return [sum(p[i] for p in parts if i < len(p)) for i in range(n)]


def concat(*parts):
    out = []
    for p in parts:
        out.extend(p)
    return out


def write(name, samples):
    peak = max(1e-9, max(abs(s) for s in samples))
    gain = min(1.0, 0.9 / peak)
    with wave.open(os.path.join(OUT, name), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(b"".join(struct.pack("<h", int(max(-1, min(1, s * gain)) * 32767)) for s in samples))


def main():
    os.makedirs(OUT, exist_ok=True)
    # Launch: a soft upward "plop".
    write("launch.wav", mix(tone(220, 520, 0.09, 0.6), noise(0.03, 0.15, seed=2, smooth=0.6)))
    # Wall bounce: a short dry tick.
    write("bounce.wav", mix(tone(1400, 1100, 0.025, 0.35, "tri", 3.0), noise(0.015, 0.2, 3.0, seed=3)))
    # Attach: a low, round thud.
    write("attach.wav", mix(tone(180, 90, 0.11, 0.7, "sine", 3.0), noise(0.04, 0.12, 3.0, seed=4, smooth=0.85)))
    # Pop: three quick bubbles climbing.
    write("pop.wav", concat(tone(600, 900, 0.045, 0.5), tone(750, 1150, 0.045, 0.5), tone(950, 1500, 0.07, 0.5)))
    # Fall: a descending whistle.
    write("fall.wav", tone(900, 260, 0.38, 0.35, "tri", 1.2, 0.01))
    # Ceiling drop: a heavy clunk with rumble.
    write("ceiling.wav", mix(tone(110, 55, 0.35, 0.8, "sine", 2.5), noise(0.25, 0.3, 2.0, seed=5, smooth=0.9)))
    # Blast: a puff of noise over a dropping boom.
    write("blast.wav", mix(noise(0.45, 0.7, 1.6, seed=6, smooth=0.5), tone(160, 40, 0.45, 0.8, "sine", 1.8)))
    # Chain accent: a shorter, higher crack for the later waves.
    write("chain.wav", mix(noise(0.18, 0.6, 2.2, seed=7, smooth=0.3), tone(320, 120, 0.18, 0.5, "tri", 2.0)))


if __name__ == "__main__":
    main()
