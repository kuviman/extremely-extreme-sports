
        export function setup_audio(audio, handler) {
            audio.oncanplaythrough = function() { handler(true); };
            audio.onerror = function() { handler(false); };
            audio.load();
        }
        