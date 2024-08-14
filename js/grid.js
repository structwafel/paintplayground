import { selectedColor } from "./color.js";

function left_or_right(button) {
    return (button === 0 || button === 2);
}

export class Grid {
    constructor() {
        this.cellSize = 10;
        this.gridWidth = 1000; // Adjust as needed
        this.gridHeight = 1000; // Adjust as needed
        this.scale = 1;
        this.translation = { x: 0, y: 0 };
        this.lastColoredSquare = null;

        this.placemouseDown = false;
        this.movemouseDown = false;
        this.lastMousePos = { x: 0, y: 0 };

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

    colorBox(index, color) {
        const box = this.gridContainer.querySelector(`[id='${index}']`);
        box.style.backgroundColor = color;
    }

    handleColoring(index) {
        if (index !== this.lastColoredSquare) {
            this.colorBox(index, selectedColor);
        }
    }

    handleZoom(event) {
        const zoomFactor = 1.02;
        const mouseX = event.clientX - this.gridContainer.getBoundingClientRect().left;
        const mouseY = event.clientY - this.gridContainer.getBoundingClientRect().top;

        if (event.deltaY < 0) {
            this.scale = Math.min(this.scale * zoomFactor, 4); // Max zoom in
            this.translation.x -= mouseX * (zoomFactor - 1);
            this.translation.y -= mouseY * (zoomFactor - 1);
        } else {
            this.scale = Math.max(this.scale / zoomFactor, 0.5); // Max zoom out
            this.translation.x += mouseX * (1 - 1 / zoomFactor);
            this.translation.y += mouseY * (1 - 1 / zoomFactor);
        }

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

        const minX = parentRect.width - gridRect.width * this.scale;
        const minY = parentRect.height - gridRect.height * this.scale;

        this.translation.x = Math.min(0, Math.max(this.translation.x, minX));
        this.translation.y = Math.min(0, Math.max(this.translation.y, minY));
    }

    applyTransformations() {
        this.gridContainer.style.transformOrigin = '0 0'; // Ensure scaling from top-left corner
        this.gridContainer.style.transform = `scale(${this.scale}) translate(${this.translation.x}px, ${this.translation.y}px)`;
    }

    addEventListeners() {
        this.gridContainer.addEventListener('click', (event) => {
            if (event.target.classList.contains('gridBox')) {
                this.handleColoring(event.target.id);
            }
        });

        this.gridContainer.addEventListener('wheel', (event) => {
            event.preventDefault();
            this.handleZoom(event);
        });

        this.gridContainer.addEventListener('mousedown', (event) => {
            if (event.button === 0) {
                this.placemouseDown = true;
            } else if (event.button == 2) {
                this.movemouseDown = true;
                this.lastMousePos = { x: event.clientX, y: event.clientY };
            }
        });

        this.gridContainer.addEventListener('mouseup', (event) => {
            if (left_or_right(event.button)) {
                this.placemouseDown = false;
                this.movemouseDown = false;
            }
        });

        document.addEventListener('mouseup', (event) => {
            if (left_or_right(event.button)) {
                this.placemouseDown = false;
                this.movemouseDown = false;
            }
        });

        this.gridContainer.addEventListener('mousemove', (event) => {
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