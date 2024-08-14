import { Grid } from '../js/grid.js';
import { selectedColor } from './color.js';
import { Ws } from './ws.js';

export class ChunkManager {
    constructor(x, y) {
        this.grid = new Grid(x, y);

        this.ws = new Ws(x, y);

        this.grid.gridContainer.addEventListener('click', (event) => {
            if (event.target.classList.contains('gridBox')) {
                this.appendColoringUpdate(event.target.id, selectedColor);
            }
        });

        this.updates = [];

        // periodically send updates to server
        setInterval(() => {
            console.log(this.updates.length);
            if (this.updates.length > 0) {
                const data = new Uint8Array(this.updates);
                const view = new DataView(data.buffer);

                this.updates.forEach((update, i) => {
                    console.log(update, i);
                    // const index = view.getUint32(i, true);
                    // const color = view.getUint8(i + 4);

                    // this.ws.socket.send(data);
                });

                // send updates as binary
                // let binary = new Uint8Array(this.updates.length * 3);
                // this.socket.send(this.updates);
                this.updates = [];
            }
        }, 1000);
    }


    appendColoringUpdate(index, color) {
        this.updates[index] = color;
    }



}