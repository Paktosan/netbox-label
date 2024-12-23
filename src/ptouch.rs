use futures_lite::future::block_on;
use nusb::transfer::RequestBuffer;
use nusb::Interface;
use std::thread::sleep;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_millis(1);
///Endpoint for receiving data from printer
const FROM_DEVICE: u8 = 0x81;
///Endpoint for sending data to device
const TO_DEVICE: u8 = 0x02;

#[derive(Debug, Eq, PartialEq)]
pub enum PrinterModel {
    P900,
    P900W,
    P950NW,
    ///We only support this, but the Brother manual also covers the other models
    P910BT,
}

#[derive(Debug)]
pub enum BatteryCharge {
    AC,
    Full,
    Half,
    Low,
    NeedToBeCharged,
}

#[derive(Debug)]
pub enum PrinterError {
    NoMedia,
    EndOfMedia,
    CutterJam,
    WeakBattery,
    HighVoltageAdapter,
    ReplaceMedia,
    ExpansionBufferFull,
    CommunicationError,
    CommunicationBufferFull,
    CoverOpen,
    Overheating,
    BlackMarkingNotDetected,
    SystemError,
    UnknownMedia,
    FleTapeEnd,
    HighResDraftPrintError,
    AdapterPullInsertError,
    IncompatibleTape,
}

#[derive(Debug)]
pub enum Status {
    StatusRequestReply,
    PrintingComplete,
    ErrorOccurred,
    TurnedOff,
    Notification,
    PhaseChange,
}

#[derive(Debug)]
pub enum Phase {
    Waiting,
    Printing,
}

#[derive(Debug)]
pub struct PrinterStatus {
    model: PrinterModel,
    ///Tape width in millimeter
    pub tape_width: u8,
    pub charge: BatteryCharge,
    pub errors: Vec<PrinterError>,
    pub status: Status,
    pub phase: Phase,
}

impl PrinterStatus {
    fn get(handle: &Interface) -> PrinterStatus {
        //P-Touch messages
        let status_request = Vec::from(0x1b6953u32.to_be_bytes());
        block_on(handle.bulk_out(TO_DEVICE, status_request))
            .into_result()
            .unwrap();
        sleep(TIMEOUT);
        let buffer = RequestBuffer::new(32);
        let response = block_on(handle.bulk_in(FROM_DEVICE, buffer))
            .into_result()
            .unwrap();
        let model: PrinterModel;
        match response[4] {
            0x78 => model = PrinterModel::P910BT,
            0x70 => model = PrinterModel::P950NW,
            0x69 => model = PrinterModel::P900W,
            0x71 => model = PrinterModel::P900,
            _ => {
                panic!("Printer Model is unknown to us!")
            }
        }
        let charge: BatteryCharge;
        if model == PrinterModel::P910BT && response[6] < 0x30 {
            match response[6] {
                0x20 => charge = BatteryCharge::Full,
                0x22 => charge = BatteryCharge::Half,
                0x23 => charge = BatteryCharge::Low,
                0x24 => charge = BatteryCharge::NeedToBeCharged,
                _ => {
                    panic!("Unknown Battery Level!")
                }
            }
        } else if model == PrinterModel::P910BT {
            charge = BatteryCharge::AC
        } else {
            panic!("This is not a P910BT, we do not support that yet!")
        }
        let mut errors: Vec<PrinterError> = Vec::new();
        match response[7] {
            0x21 => errors.push(PrinterError::UnknownMedia),
            0x10 => errors.push(PrinterError::FleTapeEnd),
            0x1d => errors.push(PrinterError::HighResDraftPrintError),
            0x1e => errors.push(PrinterError::AdapterPullInsertError),
            0 => {}
            _ => {
                panic!("Unknown extended error!")
            }
        }
        let error1 = response[8];
        let error2 = response[9];
        if error1 & 0x01 == 0x01 {
            errors.push(PrinterError::NoMedia);
        }
        if error1 & 0x02 == 0x02 {
            errors.push(PrinterError::EndOfMedia);
        }
        if error1 & 0x04 == 0x04 {
            errors.push(PrinterError::CutterJam);
        }
        if error1 & 0x08 == 0x08 {
            errors.push(PrinterError::WeakBattery);
        }
        if error1 & 0x40 == 0x40 {
            errors.push(PrinterError::HighVoltageAdapter);
        }
        if error2 & 0x01 == 0x01 {
            errors.push(PrinterError::ReplaceMedia);
        }
        if error2 & 0x02 == 0x02 {
            errors.push(PrinterError::ExpansionBufferFull);
        }
        if error2 & 0x04 == 0x04 {
            errors.push(PrinterError::CommunicationError);
        }
        if error2 & 0x08 == 0x08 {
            errors.push(PrinterError::CommunicationBufferFull);
        }
        if error2 & 0x10 == 0x10 {
            errors.push(PrinterError::CoverOpen);
        }
        if error2 & 0x20 == 0x20 {
            errors.push(PrinterError::Overheating);
        }
        if error2 & 0x40 == 0x40 {
            errors.push(PrinterError::BlackMarkingNotDetected);
        }
        if error2 & 0x80 == 0x80 {
            errors.push(PrinterError::SystemError);
        }
        let tape_width = response[10];
        if response[11] == 0xFF {
            errors.push(PrinterError::IncompatibleTape)
        }
        let status = match response[18] {
            0x00 => Status::StatusRequestReply,
            0x01 => Status::PrintingComplete,
            0x02 => Status::ErrorOccurred,
            0x04 => Status::TurnedOff,
            0x05 => Status::Notification,
            0x06 => Status::PhaseChange,
            _ => panic!("Unknown printer status!"),
        };
        let phase = match response[19] {
            0x00 => Phase::Waiting,
            0x01 => Phase::Printing,
            _ => panic!("Unknown phase!"),
        };
        PrinterStatus {
            model,
            charge,
            errors,
            tape_width,
            status,
            phase,
        }
    }
}

pub struct Printer {
    pub model: PrinterModel,
    handle: Interface,
}

impl Printer {
    pub fn init() -> Printer {
        //USB init
        let device_info = nusb::list_devices()
            .unwrap()
            .find(|dev| dev.vendor_id() == 0x04f9 && dev.product_id() == 0x20c7)
            .expect("Could not find label printer!");
        let device = device_info.open().expect("Failed to open device!");
        let handle = device
            .claim_interface(0)
            .expect("Failed to claim interface!");
        let invalidate = Vec::from([0u8; 200]); //200 bytes of nothing
        let init = Vec::from(0x1b40u16.to_be_bytes());
        block_on(handle.bulk_out(TO_DEVICE, invalidate))
            .into_result()
            .unwrap();
        block_on(handle.bulk_out(TO_DEVICE, init))
            .into_result()
            .unwrap();
        let status = PrinterStatus::get(&handle);
        if !status.errors.is_empty() {
            panic!("{:?}", status.errors)
        }
        Printer {
            model: status.model,
            handle,
        }
    }

    pub fn get_status(&self) -> PrinterStatus {
        PrinterStatus::get(&self.handle)
    }
}
