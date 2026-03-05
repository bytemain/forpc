import * as $protobuf from "protobufjs";
import Long = require("long");
/** Namespace forpc. */
export namespace forpc {

    /** StatusCode enum. */
    enum StatusCode {
        OK = 0,
        CANCELLED = 1,
        UNKNOWN = 2,
        INVALID_ARGUMENT = 3,
        DEADLINE_EXCEEDED = 4,
        NOT_FOUND = 5,
        ALREADY_EXISTS = 6,
        PERMISSION_DENIED = 7,
        RESOURCE_EXHAUSTED = 8,
        FAILED_PRECONDITION = 9,
        ABORTED = 10,
        OUT_OF_RANGE = 11,
        UNIMPLEMENTED = 12,
        INTERNAL = 13,
        UNAVAILABLE = 14,
        DATA_LOSS = 15,
        UNAUTHENTICATED = 16
    }

    /** Properties of a Call. */
    interface ICall {

        /** Call method */
        method?: (string|null);

        /** Call metadata */
        metadata?: ({ [k: string]: string }|null);
    }

    /** Represents a Call. */
    class Call implements ICall {

        /**
         * Constructs a new Call.
         * @param [properties] Properties to set
         */
        constructor(properties?: forpc.ICall);

        /** Call method. */
        public method: string;

        /** Call metadata. */
        public metadata: { [k: string]: string };

        /**
         * Creates a new Call instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Call instance
         */
        public static create(properties?: forpc.ICall): forpc.Call;

        /**
         * Encodes the specified Call message. Does not implicitly {@link forpc.Call.verify|verify} messages.
         * @param message Call message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: forpc.ICall, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Call message, length delimited. Does not implicitly {@link forpc.Call.verify|verify} messages.
         * @param message Call message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: forpc.ICall, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Call message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Call
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): forpc.Call;

        /**
         * Decodes a Call message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Call
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): forpc.Call;

        /**
         * Verifies a Call message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Call message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Call
         */
        public static fromObject(object: { [k: string]: any }): forpc.Call;

        /**
         * Creates a plain object from a Call message. Also converts values to other types if specified.
         * @param message Call
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: forpc.Call, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Call to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Call
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Status. */
    interface IStatus {

        /** Status code */
        code?: (forpc.StatusCode|null);

        /** Status message */
        message?: (string|null);
    }

    /** Represents a Status. */
    class Status implements IStatus {

        /**
         * Constructs a new Status.
         * @param [properties] Properties to set
         */
        constructor(properties?: forpc.IStatus);

        /** Status code. */
        public code: forpc.StatusCode;

        /** Status message. */
        public message: string;

        /**
         * Creates a new Status instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Status instance
         */
        public static create(properties?: forpc.IStatus): forpc.Status;

        /**
         * Encodes the specified Status message. Does not implicitly {@link forpc.Status.verify|verify} messages.
         * @param message Status message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: forpc.IStatus, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Status message, length delimited. Does not implicitly {@link forpc.Status.verify|verify} messages.
         * @param message Status message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: forpc.IStatus, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Status message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Status
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): forpc.Status;

        /**
         * Decodes a Status message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Status
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): forpc.Status;

        /**
         * Verifies a Status message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Status message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Status
         */
        public static fromObject(object: { [k: string]: any }): forpc.Status;

        /**
         * Creates a plain object from a Status message. Also converts values to other types if specified.
         * @param message Status
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: forpc.Status, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Status to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Status
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }
}
