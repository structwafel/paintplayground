export class Ws {
    constructor(x, y) {
        this.socket = this.connect_ws(x, y);
    }
    connect_ws(x, y) {
        const socket = new WebSocket(getWsUrl(x, y));
        socket.binaryType = 'arraybuffer';
        socket.onopen = function () {
            console.log('WebSocket connection established');
        };

        return socket;
    }
}