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