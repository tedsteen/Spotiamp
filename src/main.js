const { invoke } = window.__TAURI__.core;
const { message } = window.__TAURI__.dialog;
const { emit } = window.__TAURI__.event;

async function handleError(e) {
  await message(`${e}`, { title: 'Spotiamp', kind: 'error' });
}
const spotifyUrlRe = /https:\/\/open.spotify.com\/(.*)\/(.{22})/;
function spotifyUrlToUri(url) {
  const matches = spotifyUrlRe.exec(url);
  return `spotify:${matches[1]}:${matches[2]}`;
}

async function get_volume() {
  return await invoke("get_volume");
}

function preventAndStopPropagation(ev) {
  ev.preventDefault();
  ev.stopPropagation();
}
window.addEventListener("dragenter", preventAndStopPropagation);
window.addEventListener("dragover", preventAndStopPropagation);
window.addEventListener("drop", preventAndStopPropagation);

const zoom = window.innerWidth / 275.0;

document.querySelector(':root').style.setProperty('--zoom', zoom);

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
  constructor(textEl) {
    this.textEl = textEl;
    this.xOffs = 0;
  }

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

function durationToHHMMSS(ts) {
  ts = Math.floor(ts / 1000);
  const hours = Math.floor(ts / 3600);
  const minutes = Math.floor((ts - (hours * 3600)) / 60);
  const seconds = ts - (hours * 3600) - (minutes * 60);

  let timeString = hours > 0 ? hours.toString().padStart(1, '0') + ':' : "";
  timeString += minutes.toString().padStart(1, '0') + ':' +
    seconds.toString().padStart(2, '0');
  return timeString;
}

window.addEventListener("DOMContentLoaded", () => {
  const ticker = new TextTicker(document.getElementById("text"));
  let loadedUrl;
  let state = "stopped";

  async function load(url) {
    await invoke("load", { uri: spotifyUrlToUri(url) }).catch(handleError).then((trackData) => {
      loadedUrl = url;
      const timeString = durationToHHMMSS(trackData.duration);
      ticker.setText(`12. ${trackData.artist} - ${trackData.name} (${timeString})`);
    });
  }

  async function play() {
    if (loadedUrl) {
      if (state != "paused") {
        await load(loadedUrl);
      }
      await invoke("play").catch(handleError);
      state = "playing";
    }
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

  const playBtnEl = document.getElementById("main-btn-play");
  playBtnEl.addEventListener("click", play);

  const pauseBtnEl = document.getElementById("main-btn-pause");
  pauseBtnEl.addEventListener("click", pause);

  const stopBtnEl = document.getElementById("main-btn-stop");
  stopBtnEl.addEventListener("click", stop);

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

  document.addEventListener("drop", (ev) => {
    for (const item of ev.dataTransfer.items) {
      if (item.kind === "string" && item.type.match(/^text\/uri-list/)) {
        item.getAsString((url) => {
          ticker.setText(`Loading...`);
          load(url);
        });
      }
    }
  });
});
