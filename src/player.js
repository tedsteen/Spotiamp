const { invoke } = window.__TAURI__.core;
const { emit } = window.__TAURI__.event;
import { handleError, preventAndStopPropagation, SpotifyTrack, subscribeToPlaylistEvent, dispatchPlaylistEvent } from './common.js';

async function get_volume() {
  return await invoke("get_volume");
}

window.addEventListener("dragenter", preventAndStopPropagation);
window.addEventListener("dragover", preventAndStopPropagation);
window.addEventListener("drop", preventAndStopPropagation);

const characterLUT = {
  'a': [0, 0],
  'b': [0, 1],
  'c': [0, 2],
  'd': [0, 3],
  'e': [0, 4],
  'f': [0, 5],
  'g': [0, 6],
  'h': [0, 7],
  'i': [0, 8],
  'j': [0, 9],
  'k': [0, 10],
  'l': [0, 11],
  'm': [0, 12],
  'n': [0, 13],
  'o': [0, 14],
  'p': [0, 15],
  'q': [0, 16],
  'r': [0, 17],
  's': [0, 18],
  't': [0, 19],
  'u': [0, 20],
  'v': [0, 21],
  'w': [0, 22],
  'x': [0, 23],
  'y': [0, 24],
  'z': [0, 25],
  '"': [0, 26],
  '@': [0, 27],
  ' ': [0, 30],
  '0': [1, 0],
  '1': [1, 1],
  '2': [1, 2],
  '3': [1, 3],
  '4': [1, 4],
  '5': [1, 5],
  '6': [1, 6],
  '7': [1, 7],
  '8': [1, 8],
  '9': [1, 9],
  '…': [1, 10],
  '.': [1, 11],
  ':': [1, 12],
  '(': [1, 13],
  ')': [1, 14],
  '-': [1, 15],
  '\'': [1, 16],
  '!': [1, 17],
  '_': [1, 18],
  '+': [1, 19],
  '\\': [1, 20],
  '/': [1, 21],
  '[': [1, 22],
  ']': [1, 23],
  '^': [1, 24],
  '&': [1, 25],
  '%': [1, 26],
  ',': [1, 27],
  '=': [1, 28],
  '$': [1, 29],
  '#': [1, 30],
  'å': [2, 0],
  'ö': [2, 1],
  'ä': [2, 2],
  '?': [2, 3],
  '*': [2, 4],
  '<': [1, 22],
  '>': [1, 23],
  '{': [1, 22],
  '}': [1, 23]
}
class TextTicker {
  /**
   * @param {HTMLElement} textEl 
   */
  constructor(textEl) {
    this.textEl = textEl;
    this.xOffs = 0;
  }

  /**
   * @param {string} text 
   */
  setText(text) {
    if (text.length > 31) {
      text = `${text} *** ${text}`;
      this.startTicker();
    } else {
      this.stopTicker();
    }
    this.text = text;
    this.updateText();
  }

  updateText() {
    let html = "";
    let text = this.textOverride || this.text;
    for (let index in text) {
      const char = text[index].toLowerCase();
      const lut = characterLUT[char.toLowerCase()];
      html += `<div class="sprite letter-sprite" style="--letter-idx-row: ${lut[0]}; --letter-idx-col: ${lut[1]}; --letter-col: ${index}"></div>`;
    }
    this.textEl.innerHTML = html;
  }

  /**
   * @param {number} xOffs 
   */
  updateXShift(xOffs) {
    this.textEl.style.setProperty('--x-shift', `${xOffs * 5}px`);
  }

  startTicker() {
    if (!this.ticker) {
      this.ticker = setInterval(() => {
        this.xOffs = (this.xOffs + 1) % (Math.ceil(this.text.length / 2) + 2);
        if (this.textOverride === undefined) {
          this.updateXShift(-this.xOffs);
        }
      }, 220);
    }
  }

  stopTicker() {
    clearInterval(this.ticker);
    this.xOffs = 0;
    this.updateXShift(0);
  }

  /**
   * @param {string} text 
   */
  setOverride(text) {
    this.textOverride = text;
    if (this.textOverride !== undefined) {
      this.updateXShift(0);
    } else {
      this.updateXShift(-this.xOffs);
    }

    this.updateText();
  }
}

window.addEventListener("DOMContentLoaded", () => {
  const ticker = new TextTicker(document.getElementById("text"));
  ticker.setText("Winamp 2.91")

  /**
   * @type {SpotifyTrack | undefined}
   */
  let loadedTrack;

  /**
   * @type {"stopped" | "playing" | "paused"}
   */
  let state = "stopped";

  /**
   * @param {SpotifyTrack} track
   */
  function loadTrack(track) {
    ticker.setText(`${track.artist} - ${track.name} (${track.durationAsString})`);
    loadedTrack = track;
    state = "stopped";
  }

  async function play() {
    let trackToStartPlaying = loadedTrack;
    if (state == "paused") {
      trackToStartPlaying = undefined; //Don't start playing the loadedTrack, just resume the play
    }
    //console.info("PLAY:", trackToStartPlaying);
    await invoke("play", { uri: trackToStartPlaying?.uri }).catch(handleError);
    state = "playing";
  }

  async function pause() {
    if (state == "playing") {
      await invoke("pause").catch(handleError);
      state = "paused";
    }
  }

  async function stop() {
    if (state != "stopped") {
      await invoke("stop").catch(handleError);
      state = "stopped";
    }
  }

  document.getElementById("main-btn-previous").addEventListener("click", () => {
    dispatchPlaylistEvent('previous-track');
  });
  document.getElementById("main-btn-play").addEventListener("click", play);
  document.getElementById("main-btn-pause").addEventListener("click", pause);
  document.getElementById("main-btn-stop").addEventListener("click", stop);
  document.getElementById("main-btn-next").addEventListener("click", () => {
    dispatchPlaylistEvent('next-track');
  });

  const volume = document.getElementById("volume");
  get_volume().then(function (volumePct) {
    console.info("Initial volume:", volumePct);
    volume.value = volumePct;
  });

  volume.addEventListener("input", (e) => {
    const volumePct = parseInt(e.target.value);
    emit('volume-change', volumePct);
    const yOffs = -Math.floor(volumePct / 100.0 * 27) * 15 + "px";
    volume.style.backgroundPositionY = yOffs;
    ticker.setOverride(`VOLUME: ${volumePct}%`);
  });

  volume.addEventListener("mouseup", () => {
    ticker.setOverride(undefined);
  });

  // document.addEventListener("drop", (ev) => {
  //   for (const item of ev.dataTransfer.items) {
  //     if (item.kind === "string" && item.type.match(/^text\/uri-list/)) {
  //       item.getAsString((url) => {
  //         ticker.setText(`Loading...`);
  //         const uri = spotifyUrlToUri(url);
  //         getTrack(uri).then((track) => {
  //           loadTrack(track);
  //         });
  //       });
  //     }
  //   }
  // });
  subscribeToPlaylistEvent('load-track', (track) => {
    loadTrack(track);
    play().catch(handleError);
  });


});
