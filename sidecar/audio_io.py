"""Audio decoding for the sidecar, via PyAV.

PyAV's wheel carries FFmpeg's own libraries, so every format Tracet accepts
decodes without an ffmpeg binary on the user's machine (the old setup needed
Homebrew for that). whisperx.load_audio is not used because it shells out to
the ffmpeg command.
"""

import wave

import numpy as np

SAMPLE_RATE = 16000


def decode(path, on_progress=None):
    """Decode the first audio stream of any audio/video file to 16 kHz mono
    int16 samples. on_progress(percent) is called as decoding advances."""
    import av

    chunks = []
    with av.open(path) as container:
        if not container.streams.audio:
            raise ValueError("No audio track found in this file")
        stream = container.streams.audio[0]
        duration = container.duration / av.time_base if container.duration else None
        resampler = av.AudioResampler(format="s16", layout="mono", rate=SAMPLE_RATE)
        last = -1
        for frame in container.decode(stream):
            for out in resampler.resample(frame):
                chunks.append(out.to_ndarray().reshape(-1))
            if on_progress and duration and frame.time is not None:
                percent = max(0, min(99, int(frame.time / duration * 100)))
                if percent != last:
                    on_progress(percent)
                    last = percent
        for out in resampler.resample(None):
            chunks.append(out.to_ndarray().reshape(-1))

    if not chunks:
        raise ValueError("No audio could be decoded from this file")
    return np.concatenate(chunks).astype(np.int16)


def write_wav(path, samples):
    with wave.open(path, "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(SAMPLE_RATE)
        out.writeframes(samples.tobytes())


def load_audio(path):
    """Float32 samples in [-1, 1] at 16 kHz mono, as whisperX expects."""
    samples = None
    try:
        with wave.open(path, "rb") as wav:
            if (wav.getnchannels(), wav.getsampwidth(), wav.getframerate()) == (1, 2, SAMPLE_RATE):
                samples = np.frombuffer(wav.readframes(wav.getnframes()), dtype=np.int16)
    except (wave.Error, EOFError):
        pass
    if samples is None:
        samples = decode(path)
    return samples.astype(np.float32) / 32768.0
