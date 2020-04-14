import { webSocket } from "rxjs/webSocket";
import { pluck, filter, map, first } from "rxjs/operators";

const socket = webSocket(`wss://${location.host}/ws`);
socket.next("subscribed")

const not = (f) => (x) => !f(x);

const isProtocolMsg = (message) =>
    (typeof message === 'object'
        && ('welcome' in message || 'connect' in message || 'connected' in message|| 'bye' in message));

// received bye
export const byeReceived =
    socket.pipe(
        filter(({ bye }) => !!bye),
        pluck('bye'),
        first()
    );

// your peer's ID
export const connectReceived =
    socket.pipe(
        filter(({ connected }) => !!connected),
        pluck('connected'),
        first()
    );

// your ID
export const ownPeerId =
    socket.pipe(
        filter(({ welcome }) => !!welcome),
        map(({ welcome }) => welcome)
    )


export const sendAsRaw  = (payload) => socket.next(payload)

export const payloadMsg = socket
    .pipe(filter(not(isProtocolMsg)));

export const offers = socket.pipe(filter(message => typeof message === 'object' && message.type === 'offer'), pluck('payload'));
export const answers = socket.pipe(filter(message => typeof message === 'object' && message.type === 'answer'), pluck('payload'));
export const candidates = socket.pipe(filter(message => typeof message === 'object' && message.type === ' candidates', pluck('payload')));

export const sendAsType = (type) => (payload) => socket.next({type, payload})
export const sendAsOffer = sendAsType('offer');
export const sendAsAnswer = sendAsType('answer');
export const sendAsCandidate = sendAsType('candidate');

window['API'] = {
    sendAsOffer, sendAsAnswer, sendAsCandidate, offers, answers, candidates, sendAsRaw
}