

use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;
use xxv_pac::xxv_ethernet;

lazy_static! {
    pub static ref XXV_ETHERNET: Arc<Mutex<Xxvethernet>> =  Arc::new(Mutex::new(Xxvethernet::default()));
}

pub fn init() {
    
    
}

pub fn xxv_start() {
    let mut time_out = 10000000isize;
    // If already started, then there is nothing to do
    if XXV_ETHERNET.lock().is_started() {
        log::debug!("xxv ethernet is started, there is nothing to do!");
        return;
    }
    log::debug!("xxvethernet_start");
    // Enable transmitter if not already enabled
    if XXV_ETHERNET.lock().options & XXE_TRANSMITTER_ENABLE_OPTION != 0 {
        log::debug!("enabling transmitter");
        if XXV_ETHERNET.lock().hardware().txcfg.read().ctl_tx_enable().is_disable() {
            log::debug!("transmitter not enabled, enabling now");
            XXV_ETHERNET.lock().hardware().txcfg.modify(|_, w| w.ctl_tx_enable().set_bit());
        }
        log::debug!("transmitter enabled");
    }
    
    // Enable receiver
    if XXV_ETHERNET.lock().options & XXE_RECEIVER_ENABLE_OPTION != 0 {
        log::debug!("enabling receiver");
        if XXV_ETHERNET.lock().hardware().rxcfg.read().ctl_rx_enable().is_disable() {
            log::debug!("receiver not enabled, enabling now");
            XXV_ETHERNET.lock().hardware().rxcfg.modify(|_, w| w.ctl_rx_enable().set_bit());
            // RX block lock
		    // Do a dummy read because this is a sticky bit
            let _ = XXV_ETHERNET.lock().hardware().rxblsr.read().bits();
            while XXV_ETHERNET.lock().hardware().rxblsr.read().stat_rx_block_lock().is_disable() {
                time_out -= 1;
                if time_out <= 0 {
                    log::debug!("ERROR: Block lock is not set");
                    return;
                }
            }
            log::debug!("receiver enabled");
        }
    }
    // Mark as started
    XXV_ETHERNET.lock().is_started = true;
	log::debug!("XXxvEthernet_Start: done");
}

pub fn xxv_stop() {
    // If already stopped, then there is nothing to do
    if !XXV_ETHERNET.lock().is_started() {
        log::debug!("xxv ethernet is stopped, there is nothing to do!");
        return;
    }
    log::debug!("xxvethernet_stop");
    log::debug!("xxvethernet_stop: disabling receiver");
    // Disable the receiver
    XXV_ETHERNET.lock().hardware().rxcfg.modify(|_, w| w.ctl_rx_enable().clear_bit());
    // Mark as stopped
    XXV_ETHERNET.lock().is_started = false;
    log::debug!("xxvethernet_stop: done");
}

pub fn xxv_reset() {
    /*
	 * Add delay of 10000 loops to give enough time to the core to come
	 * out of reset. Till the time core comes out of reset none of the
	 * XxvEthernet registers are accessible including the IS register.
	 */
     let mut time_out_loops = XXE_LOOPS_TO_COME_OUT_OF_RST;
     while time_out_loops > 0 {
         time_out_loops -= 1;
     }
     log::debug!("xxvethernet_reset");
     // Stop the device and reset HW
     xxv_stop();
     XXV_ETHERNET.lock().options = XXE_DEFAULT_OPTIONS;
     // setup HW
     xxv_init_hw();
}

pub fn xxv_init_hw() {
    let mut reg = 0usize;
    log::debug!("xxvethernet_init_hw");
    // Disable the receiver
    XXV_ETHERNET.lock().hardware().rxcfg.modify(|_, w| w.ctl_rx_enable().set_bit());

    /*
	 * Sync default options with HW but leave receiver and transmitter
	 * disabled. They get enabled with XXxvEthernet_Start() if
	 * XXE_TRANSMITTER_ENABLE_OPTION and XXE_RECEIVER_ENABLE_OPTION
	 * are set
	 */
    let options = XXV_ETHERNET.lock().options & !(XXE_TRANSMITTER_ENABLE_OPTION | XXE_RECEIVER_ENABLE_OPTION);
    
    xxv_set_options(options);
    xxv_clear_options(!XXV_ETHERNET.lock().options);
    log::debug!("xxvethernet_init_hw: done");
}

pub fn xxv_set_options(options: usize) {
    let mut reg_rcfg = 0u32;          // Reflects original contents of RCFG
    let mut reg_tc = 0u32;            // Reflects original contents of TC
    let mut reg_new_rcfg = 0u32;      // Reflects new contents of RCFG
    let mut reg_new_tc = 0u32;        // Reflects new contents of TC
	// Be sure device has been stopped
    if XXV_ETHERNET.lock().is_started() {
        log::debug!("device is not stopped");
        return;
    }
    log::debug!("xxvethernet_set_options");
    // Set options word to its new value
    let old_options = XXV_ETHERNET.lock().options;
    XXV_ETHERNET.lock().options = old_options | options;

    // Many of these options will change the RCFG or TCFG registers.
	// To reduce the amount of IO to the device, group these options here
	// and change them all at once.
    /* Get current register contents */
    reg_rcfg = XXV_ETHERNET.lock().hardware().rxcfg.read().bits();
    reg_tc = XXV_ETHERNET.lock().hardware().txcfg.read().bits();
    reg_new_rcfg = reg_rcfg;
    reg_new_tc = reg_tc;
    log::debug!("current control regs: RCFG: {:#x}; TC: {:#x}", reg_rcfg, reg_tc);
    log::debug!("options: {:#x}; default options: {:#x}", options, XXE_DEFAULT_OPTIONS);
    // Turn on FCS stripping on receive packets
    if options & XXE_FCS_STRIP_OPTION != 0 {
        log::debug!("set_options: enabling fcs stripping");
        reg_new_rcfg |= XXE_RXCFG_DEL_FCS_MASK;
    }
	// Turn on FCS insertion on transmit packets
    if options & XXE_FCS_INSERT_OPTION != 0 {
        log::debug!("set_options: enabling fcs insertion");
        reg_new_tc |= XXE_TXCFG_FCS_MASK;
    }
    // Enable transmitter
    if options & XXE_TRANSMITTER_ENABLE_OPTION != 0 {
        reg_new_tc |= XXE_TXCFG_TX_MASK;
    }
    // Enable receiver
    if options & XXE_RECEIVER_ENABLE_OPTION != 0 {
        reg_new_rcfg |= XXE_RXCFG_RX_MASK;
    }
    // Change the TC or RCFG registers if they need to be modified
    if reg_tc != reg_new_tc {
        log::debug!("set_options: writing tc: {:#x}", reg_new_tc);
        XXV_ETHERNET.lock().hardware().txcfg.write(|w| unsafe { w.bits(reg_new_tc) });
    }
    if reg_rcfg != reg_new_rcfg {
        log::debug!("set_options: writing rcfg: {:#x}", reg_new_rcfg);
        XXV_ETHERNET.lock().hardware().rxcfg.write(|w| unsafe { w.bits(reg_new_rcfg) });
    }
    log::debug!("set_options: returning success");
}

pub fn xxv_clear_options(options: usize) {
    let mut reg_rcfg = 0u32;          // Reflects original contents of RCFG
    let mut reg_tc = 0u32;            // Reflects original contents of TC
    let mut reg_new_rcfg = 0u32;      // Reflects new contents of RCFG
    let mut reg_new_tc = 0u32;        // Reflects new contents of TC
    log::debug!("xxvethernet_clear_options: {:#x}", options);
    // Be sure device has been stopped
    if XXV_ETHERNET.lock().is_started() {
        log::debug!("device is not stopped");
        return;
    }
    // Set options word to its new value.
    let old_options = XXV_ETHERNET.lock().options;
    XXV_ETHERNET.lock().options = old_options & (!options);

    // Many of these options will change the RCFG or TC registers.
	// Group these options here and change them all at once. What we are
	// trying to accomplish is to reduce the amount of IO to the device
    reg_rcfg = XXV_ETHERNET.lock().hardware().rxcfg.read().bits();
    reg_tc = XXV_ETHERNET.lock().hardware().txcfg.read().bits();
    reg_new_rcfg = reg_rcfg;
    reg_new_tc = reg_tc;
    
    // Turn off FCS stripping on receive packets
    if options & XXE_FCS_STRIP_OPTION != 0 {
        log::debug!("xxvethernet_clear_options: disabling fcs strip");
        reg_new_rcfg &= !XXE_RXCFG_DEL_FCS_MASK;
    }
    // Turn off FCS insertion on transmit packets
    if options & XXE_FCS_INSERT_OPTION != 0 {
        log::debug!("xxvethernet_clear_options: disabling fcs insert");
        reg_new_tc &= !XXE_TXCFG_FCS_MASK;
    }
    // Disable transmitter
    if options & XXE_TRANSMITTER_ENABLE_OPTION != 0{
        log::debug!("xxvethernet_clear_options: disabling transmitter");
        reg_new_tc &= !XXE_TXCFG_TX_MASK;
    }
	// Disable receiver
    if options & XXE_RECEIVER_ENABLE_OPTION != 0 {
        log::debug!("xxvethernet_clear_options: disabling receiver");
        reg_new_rcfg &= !XXE_RXCFG_RX_MASK;
    }
    // Change the TC and RCFG registers if they need to be modified
    if reg_tc != reg_new_tc {
        log::debug!("clear_options: setting TC: {:#x}", reg_new_tc);
        XXV_ETHERNET.lock().hardware().txcfg.write(|w| unsafe { w.bits(reg_new_tc) });
    }
    if reg_rcfg != reg_new_rcfg {
        log::debug!("clear_options: setting RCFG: {:#x}", reg_new_rcfg);
        XXV_ETHERNET.lock().hardware().rxcfg.write(|w| unsafe { w.bits(reg_new_rcfg) });
    }
    log::debug!("clear_options: returning success");
}

pub fn xxv_get_options() -> usize {
    XXV_ETHERNET.lock().options
}

pub fn xxv_get_autonegspeed() -> usize {
    log::debug!("xxvethernet_get_operating_speed");
    if XXV_ETHERNET.lock().hardware().anasr.read().stat_an_lp_ability_10gbase_kr().is_enable() {
        log::debug!("xxvethernet_get_operating_speed: returning 1000");
        XXE_SPEED_10_GBPS
    } else {
        0
    }
}

pub fn xxv_set_autonegspeed() {
    log::debug!("xxvethernet_set_operating_speed");
    XXV_ETHERNET.lock().hardware().anacr.modify(|_, w| w.ctl_an_ability_10gbase_kr().set_bit());
    log::debug!("xxvethernet_set_operating_speed: done");
}

/**< XXE_FCS_STRIP_OPTION specifies the Xxv Ethernet device to strip FCS and
 *   PAD from received frames.
 *   This driver sets this option to enabled (set) by default.
 */
pub const XXE_FCS_STRIP_OPTION: usize           = 0x00000010;

/**< XXE_FCS_INSERT_OPTION specifies the Xxv Ethernet device to generate the
 *   FCS field and add PAD automatically for outgoing frames.
 *   This driver sets this option to enabled (set) by default.
 */
pub const XXE_FCS_INSERT_OPTION: usize          = 0x00000020;

/**< XXE_TRANSMITTER_ENABLE_OPTION specifies the Xxv Ethernet device
 *   transmitter to be enabled.
 *   This driver sets this option to enabled (set) by default.
 */
pub const XXE_TRANSMITTER_ENABLE_OPTION: usize  = 0x00000080;

/**< XXE_RECEIVER_ENABLE_OPTION specifies the Xxv Ethernet device receiver to
 *   be enabled.
 *   This driver sets this option to enabled (set) by default.
 */
pub const XXE_RECEIVER_ENABLE_OPTION: usize     = 0x00000100;



// XXE_DEFAULT_OPTIONS specify the options set in XXxvEthernet_Reset() and XXxvEthernet_CfgInitialize()
pub const XXE_DEFAULT_OPTIONS: usize            = 
    XXE_FCS_INSERT_OPTION | XXE_FCS_STRIP_OPTION | XXE_TRANSMITTER_ENABLE_OPTION | XXE_RECEIVER_ENABLE_OPTION;

/// The next few constants help upper layers determine the size of memory
/// pools used for Ethernet buffers and descriptor lists.


pub const XXE_MAC_ADDR_SIZE: usize  = 6;	// MAC addresses are 6 bytes
pub const XXE_MTU: usize            = 1500;	// Max MTU size of an Ethernet frame
pub const XXE_JUMBO_MTU: usize      = 8982;	// Max MTU size of a jumbo Ethernet frame
pub const XXE_HDR_SIZE: usize       = 14;	// Size of an Ethernet header
pub const XXE_TRL_SIZE: usize       = 4;	// Size of an Ethernet trailer (FCS)

pub const XXE_MAX_FRAME_SIZE: usize = (XXE_MTU + XXE_HDR_SIZE + XXE_TRL_SIZE);
pub const XXE_MAX_JUMBO_FRAME_SIZE: usize = (XXE_JUMBO_MTU + XXE_HDR_SIZE + XXE_TRL_SIZE);


pub const XXE_RX: usize             = 1;    // Receive direction
pub const XXE_TX: usize             = 2;    // Transmit direction

pub const RATE_10M: usize           = 10;
pub const RATE_100M: usize          = 100;
pub const RATE_1G: usize            = 1000;
pub const RATE_2G5: usize           = 2500;
pub const RATE_10G: usize           = 10000;


pub const XXE_LOOPS_TO_COME_OUT_OF_RST: usize   = 10000;	// Number of loops in the driver


const XXE_RXCFG_DEL_FCS_MASK: u32 = 0x00000002;
const XXE_TXCFG_FCS_MASK: u32 = 0x00000002;
const XXE_TXCFG_TX_MASK: u32 = 0x00000001;
const XXE_RXCFG_RX_MASK: u32 = 0x00000001;
const XXE_ANA_10GKR_MASK: u32 = 0x00000004;
const XXE_SPEED_10_GBPS: usize = 10;	/**< Speed of 10 Gbps */

pub struct Xxvethernet {
    base_address: usize,
    stats: usize,
    is_started: bool,
    is_ready: bool,
    options: usize,
    flag: usize,
}

impl Default for Xxvethernet {
    fn default() -> Self {
        Xxvethernet::new(crate::config::NET_DEVICE)
    }
}

impl Xxvethernet {
    pub fn new(base_address: usize) -> Self {
        Self { 
            base_address,
            stats: 0,
            is_started: false, 
            is_ready: true, 
            options: XXE_DEFAULT_OPTIONS, 
            flag: 0 
        }
    }

    fn hardware(&self) -> &xxv_ethernet::RegisterBlock {
        unsafe { &*(self.base_address as *const _) }
    }

    fn is_started(&self) -> bool {
        self.is_started
    }

    fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn is_tx_err(&self) -> bool {
        self.hardware().txsr.read().stat_tx_local_fault().is_enable()
    }

    fn is_rx_err(&self) -> bool {
        let flag = 0xFF0;                           // [4:11] 位如果存在某位为 1，则出现错误
        self.hardware().rxsr.read().bits() & flag != 0
    }

    fn get_status(&self) -> usize {
        self.hardware().sr.read().bits() as _
    }

    fn is_stats_configured(&self) -> bool {
        self.stats != 0
    }

}




