import { invoke } from "@tauri-apps/api/core";

class Bar {
    value = $state(0);
    hat = $state(0);
    hatVelocity = 0;
    gravity = 0.000005;
    levitateTimeMs = 450;
    /**
     * @param {number} index
     */
    constructor(index) {
        this.index = index;
    }

    reset() {
        this.value = 0;
        this.hat = 0;
        this.hatVelocity = 0;
    }

    /**
     * @param {number} newValue
     */
    setValue(newValue) {
        this.value = newValue;
        if (this.hat <= newValue) {
            this.hat = newValue;
            this.hatVelocity = this.gravity * this.levitateTimeMs;
        }
    }

    /**
     * @param {number} deltaTime
     */
    update(deltaTime) {
        this.hatVelocity = Math.max(
            this.hatVelocity - this.gravity * deltaTime,
            -1,
        );
        if (this.hatVelocity < 0) {
            this.hat = Math.max(0, this.hat + this.hatVelocity * deltaTime);
        }
    }
}
export class Visualizer {
    bars = $state([
        new Bar(0),
        new Bar(1),
        new Bar(2),
        new Bar(3),
        new Bar(4),
        new Bar(5),
        new Bar(6),
        new Bar(7),
        new Bar(8),
        new Bar(9),
        new Bar(10),
        new Bar(11),
        new Bar(12),
        new Bar(13),
        new Bar(14),
        new Bar(15),
        new Bar(16),
        new Bar(17),
        new Bar(18),
    ]);
    clearAfterStop = false;
    lastTick = 0;

    constructor() {
        this.running = false;
    }

    runVisualizerUpdate() {
        if (this.running) {
            const now = Date.now();
            const deltaTime = now - this.lastTick;
            this.lastTick = now;
            invoke("take_latest_spectrum", {}).then((visualizerData) => {
                if (visualizerData) {
                    let idx = 0;
                    for (var pair of visualizerData) {
                        const bar = this.bars[idx];
                        bar.setValue(Math.min(pair[1], 1));
                        bar.update(deltaTime);
                        idx++;
                    }
                }
                requestAnimationFrame(this.runVisualizerUpdate.bind(this));
            }).catch((e) => {
                console.error("Failed to fetch visualizer data", e);
                // Try again...
                requestAnimationFrame(this.runVisualizerUpdate.bind(this));
            });
        } else {
            if (this.clearAfterStop) {
                this.clear();
            }
        }
    }

    start() {
        this.running = true;
        // Start the update "loop" using requestAnimationFrame to move forward
        this.lastTick = Date.now();
        this.runVisualizerUpdate();
    }

    clear() {
        for (const bar of this.bars) {
            bar.reset();
        }
    }

    /**
     * @param {boolean} clearAfterStop
     */
    stop(clearAfterStop) {
        if (!this.running && clearAfterStop) {
            this.clear();
        } else {
            this.running = false;
            this.clearAfterStop = clearAfterStop;
        }
    }
}