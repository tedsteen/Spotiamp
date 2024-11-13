<script>
    import { onDestroy } from "svelte";
    /**
     * @typedef {Object} Props
     * @property {string} text
     * @property {string | undefined} textOverride
     * @property {boolean} unavailable
     * @property {string} x
     * @property {string} y
     */

    /** @type {Props} */
    let {
        text = $bindable(),
        textOverride = $bindable(),
        unavailable = $bindable(),
        x,
        y,
    } = $props();

    /**
     * @type {Object<String, [number, number]>}
     */
    const letterLUT = {
        a: [0, 0],
        b: [0, 1],
        c: [0, 2],
        d: [0, 3],
        e: [0, 4],
        f: [0, 5],
        g: [0, 6],
        h: [0, 7],
        i: [0, 8],
        j: [0, 9],
        k: [0, 10],
        l: [0, 11],
        m: [0, 12],
        n: [0, 13],
        o: [0, 14],
        p: [0, 15],
        q: [0, 16],
        r: [0, 17],
        s: [0, 18],
        t: [0, 19],
        u: [0, 20],
        v: [0, 21],
        w: [0, 22],
        x: [0, 23],
        y: [0, 24],
        z: [0, 25],
        '"': [0, 26],
        "@": [0, 27],
        " ": [0, 30],
        "0": [1, 0],
        "1": [1, 1],
        "2": [1, 2],
        "3": [1, 3],
        "4": [1, 4],
        "5": [1, 5],
        "6": [1, 6],
        "7": [1, 7],
        "8": [1, 8],
        "9": [1, 9],
        "…": [1, 10],
        ".": [1, 11],
        ":": [1, 12],
        "(": [1, 13],
        ")": [1, 14],
        "-": [1, 15],
        "'": [1, 16],
        "!": [1, 17],
        _: [1, 18],
        "+": [1, 19],
        "\\": [1, 20],
        "/": [1, 21],
        "[": [1, 22],
        "]": [1, 23],
        "^": [1, 24],
        "&": [1, 25],
        "%": [1, 26],
        ",": [1, 27],
        "=": [1, 28],
        $: [1, 29],
        "#": [1, 30],
        å: [2, 0],
        ö: [2, 1],
        ä: [2, 2],
        "?": [2, 3],
        "*": [2, 4],
        "<": [1, 22],
        ">": [1, 23],
        "{": [1, 22],
        "}": [1, 23],

        // Fallbacks
        ü: [0, 20],
    };

    const tickerActive = $derived(text.length > 31);

    let xShift = $state(0);
    const SEPARATOR = " *** ";
    let letters = $derived.by(() => {
        const realText = textOverride
            ? textOverride
            : tickerActive
              ? `${text}${SEPARATOR}${text}`
              : text;
        return [...realText].map((char) => {
            return letterLUT[char.toLowerCase()] || letterLUT[" "];
        });
    });

    // Reset xShift on new text
    $effect(() => {
        text; // Make sure the effect depends on the text...
        xShift = 0;
    });

    /**
     * @type {number | undefined}
     */
    let ticker;
    $effect(() => {
        clearInterval(ticker);
        if (tickerActive) {
            ticker = setInterval(() => {
                if (!textOverride) {
                    xShift = (xShift + 1) % (text.length + SEPARATOR.length);
                }
            }, 220);
        }
        xShift = 0;
    });

    onDestroy(() => clearInterval(ticker));
</script>

<div
    class="text-container"
    style:--x="{x}px"
    style:--y="{y}px"
    style:opacity={unavailable ? "50%" : "100%"}
>
    <div
        class="text-shift-container"
        style:--x-shift={textOverride ? 0 : -xShift}
    >
        {#each letters as lut, index}
            <div
                class="sprite letter-sprite"
                style:--letter-idx-row={lut[0]}
                style:--letter-idx-col={lut[1]}
                style:--letter-col={index}
            ></div>
        {/each}
    </div>
</div>

<style>
    .text-container {
        position: absolute;
        overflow: hidden;
        width: calc(31 * 5px * var(--zoom));
        height: calc(6px * var(--zoom));
        left: calc(var(--x) * var(--zoom));
        top: calc(var(--y) * var(--zoom));
    }

    .text-shift-container {
        transform: translateX(calc(var(--x-shift) * 5px * var(--zoom)));
    }

    .letter-sprite {
        --sprite-url: url(/src/static/assets/skins/base-2.91/TEXT.BMP);
        width: 5px;
        height: 6px;
        background-position: calc(-5px * var(--letter-idx-col))
            calc(-6px * var(--letter-idx-row));
        --sprite-x: calc(var(--letter-col) * 5px);
    }
</style>
