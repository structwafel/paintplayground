import { Grid } from '../js/grid.js';
import { colorMapping } from './color.js';
import { Ws } from './ws.js';

export class ChunkManager {
    constructor(x, y) {
        this.grid = new Grid(this.appendColoringUpdate.bind(this));

        this.ws = new Ws(x, y, this.applyColoringUpdate.bind(this));


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
    }


    appendColoringUpdate(index, color) {
        this.updates[index] = color;
    }

    applyColoringUpdate(index, color) {
        this.grid.colorBox(index, color);
    }


}