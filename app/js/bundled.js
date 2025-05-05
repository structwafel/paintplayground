/* Bundled JS file generated on  */

/* File: js/canvas.js */

let mouseDown = false;
let changedSquare = false;

export class CanvasManager {
    constructor(x, y) {
        this.socket = this.connect_ws(x, y);

        const canvas = document.getElementById('colorGrid');
        canvas.width = 1000;
        canvas.height = 1000;

        canvas.addEventListener('click', this.handle_coloring.bind(this));
        canvas.addEventListener('mousedown', (event) => {
            mouseDown = true;
            this.lastMousePos = { x: event.clientX, y: event.clientY };
        });
        canvas.addEventListener('mouseup', () => mouseDown = false);
        canvas.addEventListener('mousemove', (event) => {
            if (mouseDown) {
                this.handle_coloring(event);
                this.handle_panning(event);
            }
        });
        canvas.addEventListener('wheel', this.handle_zoom.bind(this));
        console.log("added event listener", canvas);
        this.canvas = canvas;

        const ctx = canvas.getContext('2d');
        this.ctx = ctx;

        this.cellSize = 10;
        this.gridWidth = 100;
        this.gridHeight = 100;
        this.colors = new Array(this.gridWidth * this.gridHeight).fill('grey');

        // Track the last colored square
        this.lastColoredSquare = { x: null, y: null };

        // Track transformations
        this.scale = 1;
        this.translation = { x: 0, y: 0 };
    }

    connect_ws(x, y) {
        const socket = new WebSocket(getWsUrl(x, y));
        socket.binaryType = 'arraybuffer';
        socket.onopen = function () {
            console.log('WebSocket connection established');
        };

        return socket;
    }

    draw_sqare(x, y, color) {
        this.ctx.fillStyle = color;
        this.ctx.fillRect(x * this.cellSize, y * this.cellSize, this.cellSize, this.cellSize);
    }

    draw_entire_grid() {
        this.ctx.save();
        this.ctx.setTransform(this.scale, 0, 0, this.scale, this.translation.x, this.translation.y);
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

        for (let i = 0; i < this.gridWidth; i++) {
            for (let j = 0; j < this.gridHeight; j++) {
                this.draw_sqare(i, j, this.colors[i * this.gridWidth + j]);
            }
        }

        this.ctx.restore();
    }

    handle_coloring(event) {
        const rect = this.canvas.getBoundingClientRect();
        const x = Math.floor((event.clientX - rect.left - this.translation.x) / (this.cellSize * this.scale));
        const y = Math.floor((event.clientY - rect.top - this.translation.y) / (this.cellSize * this.scale));

        // Check if the current square is different from the last colored square
        if (x !== this.lastColoredSquare.x || y !== this.lastColoredSquare.y) {
            console.log("handling coloring");
            // Update the last colored square
            this.lastColoredSquare = { x, y };

            // Send the color to the server
            const color = selectedColor;
            this.colors[y * this.gridWidth + x] = color;
            this.draw_sqare(x, y, color);
            // this.socket.send(JSON.stringify({ x, y, color }));
        }
    }

    handle_zoom(event) {

        event.preventDefault();
        console.log("handling zoom");
        const zoomFactor = 1.1;
        const mouseX = event.clientX - this.canvas.getBoundingClientRect().left;
        const mouseY = event.clientY - this.canvas.getBoundingClientRect().top;

        if (event.deltaY < 0) {
            // Zoom in
            const newScale = this.scale * zoomFactor;
            console.log(newScale);
            if (newScale > 1) {
                this.scale = 1;
            } else {
                this.scale = newScale;
            }
            this.translation.x -= mouseX * (zoomFactor - 1);
            this.translation.y -= mouseY * (zoomFactor - 1);
        } else {
            // Zoom out

            const newScale = this.scale * zoomFactor;
            console.log(newScale);
            if (newScale < 1) {
                this.scale = 1;
            } else {
                this.scale = newScale;
            }
            this.translation.x += mouseX * (1 - 1 / zoomFactor);
            this.translation.y += mouseY * (1 - 1 / zoomFactor);
        }

        this.draw_entire_grid();
    }

    handle_panning(event) {
        // return if left mouse button is pressed
        if (event.buttons === 1) {
            return;
        }
        const dx = event.clientX - this.lastMousePos.x;
        const dy = event.clientY - this.lastMousePos.y;

        this.translation.x += dx;
        this.translation.y += dy;

        this.lastMousePos = { x: event.clientX, y: event.clientY };

        this.draw_entire_grid();
    }
}

/* File: js/cell.js */

// return a BigUing64 with a 60bit index and 4bit color
export function packedCell(index, color) {

    return BigInt(index) << 4n | BigInt(color);
}

/* File: js/utils.js */


/* File: js/grid.js */

function left_or_right(button) {
    return (button === 0 || button === 2);
}

const defaultGridScale = 0.5;
const borderThickness = 30;
export class Grid {
    constructor(coloringCallback) {
        this.cellSize = 10;
        this.gridWidth = 2000;
        this.gridHeight = 2000;
        this.scale = defaultGridScale;
        this.translation = { x: 0, y: 0 };
        this.lastColoredSquare = null;

        this.placemouseDown = false;
        this.movemouseDown = false;
        this.lastMousePos = { x: 0, y: 0 };


        this.coloringCallback = coloringCallback;

        this.createGrid();
        this.addEventListeners();
    }

    createGrid() {
        const gridContainer = document.getElementById('gridContainer');
        gridContainer.style.position = 'absolute';
        gridContainer.style.overflow = 'hidden';
        gridContainer.style.width = `${this.gridWidth}px`;
        gridContainer.style.height = `${this.gridHeight}px`;

        for (let i = 0; i < this.gridWidth / this.cellSize; i++) {
            for (let j = 0; j < this.gridHeight / this.cellSize; j++) {
                const box = document.createElement('div');
                const index = i * (this.gridWidth / this.cellSize) + j;
                box.id = index;
                box.className = 'gridBox';
                gridContainer.appendChild(box);
            }
        }

        this.gridContainer = gridContainer;
    }

    clear() {
        const boxes = this.gridContainer.querySelectorAll('.gridBox');
        boxes.forEach(box => box.style.backgroundColor = 'grey');
    }


    colorBox(index, color) {
        const box = this.gridContainer.querySelector(`[id='${index}']`);

        if (!box) {
            return
        }
        box.style.backgroundColor = color;
    }

    handleColoring(index) {
        if (index !== this.lastColoredSquare) {
            this.coloringCallback(index, selectedColor);
            this.colorBox(index, colorFromNumber(colorMapping[selectedColor]));
        }
    }

    handleZoom(event) {
        const zoomFactor = 1.02;
        const mouseX = event.clientX - this.gridContainer.getBoundingClientRect().left;
        const mouseY = event.clientY - this.gridContainer.getBoundingClientRect().top;
        const prevScale = this.scale;

        if (event.deltaY < 0) {
            this.scale = Math.min(this.scale * zoomFactor, 4); // Max zoom in
        } else {
            this.scale = Math.max(this.scale / zoomFactor, defaultGridScale); // Max zoom out
        }

        const scaleChange = this.scale / prevScale;

        // Adjust translation to keep the zoom centered around the mouse position
        this.translation.x = mouseX - scaleChange * (mouseX - this.translation.x);
        this.translation.y = mouseY - scaleChange * (mouseY - this.translation.y);

        this.constrainTranslation();
        this.applyTransformations();
    }

    handlePanning(event) {
        if (!this.movemouseDown) return;

        const dx = event.clientX - this.lastMousePos.x;
        const dy = event.clientY - this.lastMousePos.y;

        this.translation.x += dx;
        this.translation.y += dy;

        this.constrainTranslation();
        this.lastMousePos = { x: event.clientX, y: event.clientY };

        this.applyTransformations();
    }

    constrainTranslation() {
        const parentRect = this.gridContainer.parentElement.getBoundingClientRect();
        const gridRect = this.gridContainer.getBoundingClientRect();

        const scaledWidth = this.gridWidth * this.scale;
        const scaledHeight = this.gridHeight * this.scale;

        const minX = parentRect.width - scaledWidth - borderThickness;
        const minY = parentRect.height - scaledHeight - borderThickness;

        if (this.scale <= defaultGridScale) {
            this.translation.x = Math.min(borderThickness, Math.max(this.translation.x, minX));
            this.translation.y = Math.min(borderThickness, Math.max(this.translation.y, minY));
        } else {
            const maxX = borderThickness;
            const maxY = borderThickness;
            this.translation.x = Math.min(maxX, Math.max(this.translation.x, minX));
            this.translation.y = Math.min(maxY, Math.max(this.translation.y, minY));
        }
    }

    applyTransformations() {
        this.gridContainer.style.transformOrigin = '0 0'; // Ensure scaling from top-left corner
        this.gridContainer.style.transform = `translate(${this.translation.x}px, ${this.translation.y}px) scale(${this.scale})`;
    }

    addEventListeners() {
        this.gridContainer.addEventListener('click', (event) => {
            event.preventDefault();
            if (event.target.classList.contains('gridBox')) {
                this.handleColoring(event.target.id);
            }
        });

        this.gridContainer.addEventListener('wheel', (event) => {
            event.preventDefault();
            this.handleZoom(event);
        });

        this.gridContainer.addEventListener('mousedown', (event) => {
            event.preventDefault();
            if (event.button === 0) {
                this.placemouseDown = true;
            } else if (event.button == 2) {
                this.movemouseDown = true;
                this.lastMousePos = { x: event.clientX, y: event.clientY };
            }
        });

        this.gridContainer.addEventListener('mouseup', (event) => {
            event.preventDefault();
            if (left_or_right(event.button)) {
                this.placemouseDown = false;
                this.movemouseDown = false;
            }
        });

        document.addEventListener('mouseup', (event) => {
            event.preventDefault();
            if (left_or_right(event.button)) {
                this.placemouseDown = false;
                this.movemouseDown = false;
            }
        });

        this.gridContainer.addEventListener('mousemove', (event) => {
            event.preventDefault();
            if (this.placemouseDown) {
                this.handleColoring(event.target.id);
            } else if (this.movemouseDown) {
                this.handlePanning(event);
            }
        });

        this.gridContainer.addEventListener('contextmenu', (event) => {
            event.preventDefault();
        });
    }
}

/* File: js/manager.js */

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


/* File: js/ws.js */

export class Ws {
    constructor(x, y, applyColoringUpdate) {
        this.x = x;
        this.y = y;

        this.applyColor = applyColoringUpdate;

        this.reconnectDelay = 1000; // initial delay

        this.socket = this.connect_ws(x, y);
    }

    connect_ws(x, y) {
        const socket = new WebSocket(getWsUrl(x, y));
        socket.binaryType = 'arraybuffer';
        socket.onopen = () => {
            this.updateConnectionStatus('green', 'Connected');
            console.log('WebSocket connection established');
        };

        socket.onmessage = (event) => {
            this.handleMessage(event.data);
        }

        socket.onclose = () => {
            console.log('WebSocket connection closed, attempting to reconnect...');
            this.updateConnectionStatus('orange', 'Reconnecting...');
            this.reconnect();
        };

        socket.onerror = () => {
            console.log('WebSocket encountered an error');
            this.updateConnectionStatus('red', 'Error');
        };

        return socket;
    }

    reconnect() {
        setTimeout(() => {
            console.log('Reconnecting...');
            this.socket = this.connect_ws(this.x, this.y);
            this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000); // Exponential backoff, max 30 seconds
        }, this.reconnectDelay);
    }


    connect_and_extrac_ws(x, y) {
        const socket = this.connect_ws(x, y);
        socket.onmessage = (event) => {
            this.handleMessage(event.data);
        }


        return socket;
    }

    // todo do this server side, so it checks if you are allowed to move to that square
    move(x, y) {
        this.socket.close();

        this.x += x;
        this.y += y;

        this.socket = this.connect_and_extrac_ws(this.x, this.y);
    }

    handleMessage(data) {
        // console.log('Received message with length', data.byteLength / 8);
        // all will be binary.
        const view = new DataView(data);
        const messageType = view.getUint8(0);
        switch (messageType) {
            // receiving the entire chunk

            case 1:
                console.log('Received chunk');
                for (let i = 1; i < data.byteLength; i++) {
                    const byte = view.getUint8(i);
                    const color1 = byte >> 4;
                    const color2 = byte & 0x0F;

                    const doublePackedColor1 = colorFromNumber(color1);
                    const doublePackedColor2 = colorFromNumber(color2);

                    this.applyColor((i - 1) * 2, doublePackedColor1);
                    this.applyColor((i - 1) * 2 + 1, doublePackedColor2);
                }
                break;
            // chunks updates, a packed u64 with 60bit index and 4bit color
            case 2:
                console.log('Received chunk updates');
                for (let i = 1; i < data.byteLength; i += 8) {
                    const packed = view.getBigUint64(i, true);
                    const index = Number(packed >> 4n);

                    const colorNumber = Number(packed & 0b1111n);

                    this.applyColor(index, colorFromNumber(colorNumber));
                }
                break
            // chunk not found
            // requested a chunk that does not exist, disconnectm
            case 3:
                console.error('Chunk not found');
                this.socket.close();
                break;
            // too many chunks loaded, wait for a bit
            case 4:
                alert('Too many chunks loaded, wait a bit');
                this.socket.close();
                break;
            default:
                console.error('Unknown message type');
        }
    }

    updateConnectionStatus(color, text) {
        const statusDiv = document.getElementById('connection_status');
        if (statusDiv) {
            statusDiv.style.backgroundColor = color;
            statusDiv.textContent = text;
        }
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

/* File: js/color.js */
export let selectedColor = 'one';
export const colorMapping = {
    'zero': 0,
    'one': 1,
    'two': 2,
    'three': 3,
    'four': 4,
    'five': 5,
    'six': 6,
    'seven': 7,
    'eight': 8,
    'nine': 9,
    'ten': 10,
    'eleven': 11,
    'twelve': 12,
    'thirteen': 13,
    'fourteen': 14,
    'fifteen': 15
};
export function colorFromNumber(number) {
    switch (number) {
        case 0:
            return "#e0d3c8";
        case 1:
            return '#f5eeb0';
        case 2:
            return '#fabf61';
        case 3:
            return '#e08d51';
        case 4:
            return '#8a5865';
        case 5:
            return '#452b3f';
        case 6:
            return '#2c5e3b';
        case 7:
            return '#609c4f';
        case 8:
            return '#c6cc54';
        case 9:
            return '#78c2d6';
        case 10:
            return '#5479b0';
        case 11:
            return '#56546e';
        case 12:
            return '#839fa6';
        case 13:
            return '#f05b5b';
        case 14:
            return '#8f325f';
        case 15:
            return '#eb6c98';
        default:
            return '#e0d3c8';
    }
}
document.getElementById('color-picker').addEventListener('click', function (event) {
    selectedColor = event.target.id;
});

// Set the background color of the color buttons
Object.keys(colorMapping).forEach(key => {
    const button = document.getElementById(key);
    if (button) {
        button.style.backgroundColor = colorFromNumber(colorMapping[key]);
    }
});

/* File: js/stuff.js */

window.chunk = new ChunkManager(0, 0);

