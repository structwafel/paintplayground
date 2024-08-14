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

export const host = window.location.host;
// if it is secure use wss
export function getWsUrl(x, y) {
    if (window.location.protocol === 'https:') {
        return `wss://${host}/ws/${x}/${y}`;
    } else {
        return `ws://${host}/ws/${x}/${y}`;
    }
}
