class ChunkManager {
    constructor(x, y) {
        this.socket = this.connect_ws();


    }

    connect_ws(x, y) {
        socket = new WebSocket(websocketUrl);
        socket.binaryType = 'arraybuffer';
        socket.onopen = function () {
            console.log('WebSocket connection established');
        };
        return socket;
    }
}