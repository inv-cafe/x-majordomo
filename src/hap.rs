// std
use std::net::{IpAddr, Ipv4Addr};
// crates.io
use hap::{
	accessory::{AccessoryCategory, AccessoryInformation, HapAccessory},
	characteristic::AsyncCharacteristicCallbacks,
	futures::FutureExt,
	server::{IpServer, Server},
	service::{
		accessory_information::AccessoryInformationService, switch::SwitchService, HapService,
	},
	storage::{FileStorage, Storage},
	Config, HapType, MacAddress, Pin,
};
use serde::{ser::SerializeStruct, Serialize, Serializer};
// pdu
use crate::{mqtt::MqttMessage, prelude::*, OneshotRx, Tx};

const HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

#[derive(Debug)]
struct Pdu {
	id: u64,
	information: AccessoryInformationService,
	// voltage: ?,
	switch0: SwitchService,
	switch1: SwitchService,
	switch2: SwitchService,
	switch3: SwitchService,
	switch4: SwitchService,
	switch5: SwitchService,
	switch6: SwitchService,
	switch7: SwitchService,
}
impl Pdu {
	const ID: u64 = 1;
	const NAME: &str = "Pdu";

	fn switch_of(id: u64, hap_tx: Tx) -> SwitchService {
		let mut s = SwitchService::new(id, Self::ID);

		s.power_state.on_update_async(Some(move |_, v| {
			let hap_tx = hap_tx.clone();

			async move {
				const OFF: u8 = 2;
				const ON: u8 = 3;

				let topic = "/yespeed/pdu/yespeed/19847786504205500130392/in/1000101".into();
				let id = id / 7;

				// err
				if v {
					let _ = hap_tx
						.send(MqttMessage {
							topic,
							payload: format!("{{\"devid\":1,\"linid\":{id},\"actid\":{ON}}}"),
						})
						.await;
				} else {
					let _ = hap_tx
						.send(MqttMessage {
							topic,
							payload: format!("{{\"devid\":1,\"linid\":{id},\"actid\":{OFF}}}"),
						})
						.await;
				}

				Ok(())
			}
			.boxed()
		}));

		s
	}
}
impl HapAccessory for Pdu {
	fn get_id(&self) -> u64 {
		self.id
	}

	fn set_id(&mut self, id: u64) {
		self.id = id;
	}

	fn get_service(&self, hap_type: HapType) -> Option<&dyn HapService> {
		self.get_services().into_iter().find(|&service| service.get_type() == hap_type)
	}

	fn get_mut_service(&mut self, hap_type: HapType) -> Option<&mut dyn HapService> {
		self.get_mut_services().into_iter().find(|service| service.get_type() == hap_type)
	}

	fn get_services(&self) -> Vec<&dyn HapService> {
		vec![
			&self.information,
			// &self.voltage,
			&self.switch0,
			&self.switch1,
			&self.switch2,
			&self.switch3,
			&self.switch4,
			&self.switch5,
			&self.switch6,
			&self.switch7,
		]
	}

	fn get_mut_services(&mut self) -> Vec<&mut dyn HapService> {
		vec![
			&mut self.information,
			// &mut self.voltage,
			&mut self.switch0,
			&mut self.switch1,
			&mut self.switch2,
			&mut self.switch3,
			&mut self.switch4,
			&mut self.switch5,
			&mut self.switch6,
			&mut self.switch7,
		]
	}
}
impl Serialize for Pdu {
	fn serialize<S: Serializer>(&self, serializer: S) -> StdResult<S::Ok, S::Error> {
		let mut s = serializer.serialize_struct("HapAccessory", 2)?;

		s.serialize_field("aid", &self.get_id())?;
		s.serialize_field("services", &self.get_services())?;

		s.end()
	}
}

pub async fn start(hap_tx: Tx, hap_rx: OneshotRx) -> Result<()> {
	let mut storage = FileStorage::current_dir().await?;
	let pdu = Pdu {
		id: 1,
		information: AccessoryInformation { name: Pdu::NAME.into(), ..Default::default() }
			.to_service(1, Pdu::ID)?,
		// voltage: TemperatureSensorService::new(63, Pdu::ID),
		switch0: Pdu::switch_of(7, hap_tx.clone()),
		switch1: Pdu::switch_of(14, hap_tx.clone()),
		switch2: Pdu::switch_of(21, hap_tx.clone()),
		switch3: Pdu::switch_of(28, hap_tx.clone()),
		switch4: Pdu::switch_of(35, hap_tx.clone()),
		switch5: Pdu::switch_of(42, hap_tx.clone()),
		switch6: Pdu::switch_of(49, hap_tx.clone()),
		switch7: Pdu::switch_of(56, hap_tx),
	};
	let config = if let Ok(mut c) = storage.load_config().await {
		c.host = HOST;
		storage.save_config(&c).await?;

		c
	} else {
		let c = Config {
			host: HOST,
			pin: Pin::new([0, 0, 0, 0, 0, 0, Pdu::ID as _, 0])?,
			name: Pdu::NAME.into(),
			device_id: MacAddress::from([0, 0, 0, 0, Pdu::ID as _, 0]),
			category: AccessoryCategory::Sensor,
			..Default::default()
		};

		storage.save_config(&c).await?;

		c
	};
	let server = IpServer::new(config, storage).await?;
	let pdu_ptr = server.add_accessory(pdu).await?;
	let service = server.run_handle();
	let state = hap_rx.await?;
	let switches = &state[0]["subdevs"];

	for (i, s) in pdu_ptr
		.lock()
		.await
		.get_mut_services()
		.into_iter()
		.filter_map(|service| service.get_mut_characteristic(HapType::PowerState))
		.enumerate()
	{
		if switches[i]["on"].as_u64().unwrap() == 1 {
			s.set_value(true.into()).await?;
		} else {
			s.set_value(false.into()).await?;
		}
	}

	service.await.map_err(|e| anyhow::anyhow!("{e}"))
}
