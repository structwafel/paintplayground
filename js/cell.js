
// return a BigUing64 with a 60bit index and 4bit color
export function packedCell(index, color) {

    return BigInt(index) << 4n | BigInt(color);
}