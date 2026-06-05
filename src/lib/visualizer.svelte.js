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
    bars = $state(Array.from({ length: 19 }, (_, index) => new Bar(index)));
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
                if (Array.isArray(visualizerData)) {
                    visualizerData.forEach((pair, index) => {
                        const bar = this.bars[index];
                        if (!bar) {
                            return;
                        }
                        bar.setValue(Math.min(pair[1], 1));
                        bar.update(deltaTime);
                    });
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
