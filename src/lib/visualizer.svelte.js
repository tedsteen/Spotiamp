import { invoke } from "@tauri-apps/api/core";

class Bar {
    current = $state(0);
    hat = $state(0);
    hatVelocity = 0;
    gravity = 0.000007;
    /**
     * @param {number} index
     */
    constructor(index) {
        this.index = index;
    }

    reset() {
        this.current = 0;
        this.hat = 0;
        this.hatVelocity = 0;
    }

    /**
     * @param {number} newValue
     */
    setValue(newValue) {
        this.current = newValue;
        if (this.hat <= newValue) {
            this.hat = newValue;
            this.hatVelocity = 0.005;
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

    constructor() {
        this.running = false;
    }

    async start() {
        this.running = true;

        let lastTick = Date.now();

        while (this.running) {
            try {
                const now = Date.now();
                const deltaTime = now - lastTick;

                const visualizerData = await invoke("take_latest_spectrum", {});
                lastTick = now;
                if (visualizerData) {
                    let idx = 0;
                    for (var pair of visualizerData) {
                        const bar = this.bars[idx];
                        bar.setValue(Math.min(pair[1], 1));
                        bar.update(deltaTime);
                        idx++;
                    }
                }
            } catch (e) {
                console.error("Failed to fetch visualizer data", e);
            }
        }
    }
    stop() {
        this.running = false;
    }
}