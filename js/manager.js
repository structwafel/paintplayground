import { Grid } from '../js/grid.js';
import { getWsUrl } from '../js/utils.js';

export class ChunkManager {
    constructor(x, y) {
        this.grid = new Grid(x, y);

        this.socket = this.connect_ws(x, y);

        this.grid.gridContainer.addEventListener('click', (event) => {
            if (event.target.classList.contains('gridBox')) {
                const x = event.target.dataset.x;
                const y = event.target.dataset.y;
                this.sendColoringMessage(x, y);
            }
        });
    }

    connect_ws(x, y) {
        const socket = new WebSocket(getWsUrl(x, y));
        socket.binaryType = 'arraybuffer';
        socket.onopen = function () {
            console.log('WebSocket connection established');
        };

        return socket;
    }

    sendColoringMessage(x, y) {
        const color = 'blue'; // Replace with selectedColor
        const message = JSON.stringify({ x, y, color });
        this.socket.send(message);
    }
}