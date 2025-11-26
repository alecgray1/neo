// Neo Plugin SDK - Helper Functions
// These work alongside the Neo global object injected by the runtime

/**
 * Create a Real point value
 * @param {number} value
 * @returns {{Real: number}}
 */
export function real(value) {
    return { Real: value };
}

/**
 * Create an Unsigned point value
 * @param {number} value
 * @returns {{Unsigned: number}}
 */
export function unsigned(value) {
    return { Unsigned: value };
}

/**
 * Create a Boolean point value
 * @param {boolean} value
 * @returns {{Boolean: boolean}}
 */
export function boolean(value) {
    return { Boolean: value };
}

/**
 * Create an Enumerated point value
 * @param {number} value
 * @returns {{Enumerated: number}}
 */
export function enumerated(value) {
    return { Enumerated: value };
}

/**
 * Create a Null point value
 * @returns {"Null"}
 */
export function nullValue() {
    return "Null";
}

/**
 * Extract a number from a PointValue (Real or Unsigned)
 * @param {*} value
 * @returns {number|undefined}
 */
export function asNumber(value) {
    if (typeof value === "object" && value !== null) {
        if ("Real" in value) return value.Real;
        if ("Unsigned" in value) return value.Unsigned;
        if ("Enumerated" in value) return value.Enumerated;
    }
    return undefined;
}

/**
 * Extract a boolean from a PointValue
 * @param {*} value
 * @returns {boolean|undefined}
 */
export function asBoolean(value) {
    if (typeof value === "object" && value !== null && "Boolean" in value) {
        return value.Boolean;
    }
    return undefined;
}

/**
 * Check if a PointValue is Null
 * @param {*} value
 * @returns {boolean}
 */
export function isNull(value) {
    return value === "Null";
}

/**
 * Create an OK response
 * @returns {{type: "Ok"}}
 */
export function okResponse() {
    return { type: "Ok" };
}

/**
 * Create an error response
 * @param {string} code
 * @param {string} message
 * @returns {{type: "Error", code: string, message: string}}
 */
export function errorResponse(code, message) {
    return { type: "Error", code, message };
}

/**
 * Create a custom response with payload
 * @param {*} payload
 * @returns {{type: "Custom", payload: *}}
 */
export function customResponse(payload) {
    return { type: "Custom", payload };
}
