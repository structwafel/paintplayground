import { Grid } from '../js/grid.js';
import { colorMapping } from './color.js';
import { Ws } from './ws.js';

export class ChunkManager {
    constructor(x, y) {
        this.grid = new Grid(this.appendColoringUpdate.bind(this));

        this.ws = new Ws(x, y, this.applyColoringUpdate.bind(this));

        this.allowUpdates = true;
        this.updates = [];

        // periodically send updates to server
        setInterval(() => {
            const filteredUpdates = this.updates
                .map((color, index) => ({ index, color }))
                .filter(update => update.color !== undefined);

            if (filteredUpdates.length > 0) {
                console.log("sending", filteredUpdates.length, "updates");
                const data = new Uint8Array(filteredUpdates.length * 8); // Each u64 is 8 bytes
                const view = new DataView(data.buffer);

                filteredUpdates.forEach((update, i) => {
                    view.setBigUint64(i * 8, BigInt(update.index << 4) | BigInt(colorMapping[update.color]), true);
                });

                this.ws.socket.send(data.buffer);
                this.updates = [];
            }
        }, 1000);

        this.addEventsToNavigationButtons();
    }


    appendColoringUpdate(index, color) {
        if (this.allowUpdates) {
            this.updates[index] = color;
        }
    }

    applyColoringUpdate(index, color) {
        if (this.allowUpdates) {
            this.grid.colorBox(index, color);
        }
    }

    addEventsToNavigationButtons() {
        const up = document.getElementById("upButton");
        const right = document.getElementById("rightButton");
        const down = document.getElementById("downButton");
        const left = document.getElementById("leftButton");
        if (up) {
            up.addEventListener("click", () => {
                this.buttonEvent(0, 1);
            });
        }
        if (down) {
            down.addEventListener("click", () => {
                this.buttonEvent(0, -1);
            });
        }
        if (left) {
            left.addEventListener("click", () => {
                this.buttonEvent(-1, 0);
            });
        }
        if (right) {
            right.addEventListener("click", () => {
                this.buttonEvent(1, 0);
            });
        }
    }

    buttonEvent(x, y) {
        console.log("moving ", x, y);
        this.allowUpdates = false;
        this.updates = [];
        this.grid.clear();

        this.ws.move(x, y);
        this.changeLocationText(this.ws.x, this.ws.y);
        this.changeDownloadText(this.ws.x, this.ws.y);
        this.allowUpdates = true;
    }

    changeLocationText(x, y) {
        const locationText = document.getElementById("location");
        locationText.innerText = `Current location: (${x}, ${y})`;
    }
    changeDownloadText(x, y) {
        const downloadLink = document.getElementById("downloadLink");
        downloadLink.href = `https://canvas.structwafel.dev/screenshot?x=${x}&y=${y}`;
    }
}

