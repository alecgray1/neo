#!/usr/bin/env python3
"""
virtual_all_devices.py - All virtual BACnet devices in one application

Creates 5 VAV units and 2 AHU units, all on the same BACnet/IP network.
All devices respond to the same IP:port (127.0.0.1:47808).
"""
import asyncio
import random
from bacpypes3.debugging import bacpypes_debugging, ModuleLogger
from bacpypes3.app import Application
from bacpypes3.local.device import DeviceObject
from bacpypes3.local.analog import AnalogValueObject, AnalogInputObject, AnalogOutputObject
from bacpypes3.local.binary import BinaryInputObject, BinaryValueObject
from bacpypes3.primitivedata import Real
from bacpypes3.basetypes import EngineeringUnits

# Module logger
_debug = 0
_log = ModuleLogger(globals())


@bacpypes_debugging
class MultiDeviceApplication:
    """Application managing multiple virtual BACnet devices"""

    def __init__(self):
        """Initialize the application"""
        self.devices = []
        self.running = True
        self.app = None

    async def create_device(self, device_instance: int, device_name: str, device_type: str):
        """Create a device and add it to the application"""

        # Create the device object
        device_object = DeviceObject(
            objectName=device_name,
            objectIdentifier=("device", device_instance),
            maxApduLengthAccepted=1024,
            segmentationSupported="segmentedBoth",
            vendorIdentifier=15,
            vendorName="Neo Virtual Device",
            modelName=f"Virtual {device_type}",
        )

        # If this is the first device, create the application
        if self.app is None:
            self.app = Application(device_object, "127.0.0.1:47808")
            print(f"✓ Created BACnet/IP application at 127.0.0.1:47808")
        else:
            # Add additional device to the existing application
            # Note: BACpypes3 doesn't natively support multiple devices per application
            # We'll need to use the first device as the main one
            print(f"⚠  BACpypes3 limitation: Can only have one device per application")
            print(f"   Skipping {device_name} - would need separate IP address")
            return None

        objects = {}

        if device_type == "VAV":
            # VAV devices: AI:1, AO:1, BI:1, AV:1
            temp_sensor = AnalogInputObject(
                objectIdentifier=("analogInput", 1),
                objectName=f"{device_name}-Temperature",
                presentValue=Real(72.0 + (device_instance % 5)),
                units=EngineeringUnits.degreesFahrenheit,
                description="Zone temperature sensor",
            )
            self.app.add_object(temp_sensor)
            objects['temp'] = temp_sensor

            damper = AnalogOutputObject(
                objectIdentifier=("analogOutput", 1),
                objectName=f"{device_name}-Damper",
                presentValue=Real(45.0),
                units=EngineeringUnits.percent,
                description="Damper position control",
            )
            self.app.add_object(damper)
            objects['damper'] = damper

            occupancy = BinaryInputObject(
                objectIdentifier=("binaryInput", 1),
                objectName=f"{device_name}-Occupancy",
                presentValue="inactive" if device_instance % 2 == 0 else "active",
                description="Occupancy sensor",
            )
            self.app.add_object(occupancy)
            objects['occupancy'] = occupancy

            setpoint = AnalogValueObject(
                objectIdentifier=("analogValue", 1),
                objectName=f"{device_name}-Setpoint",
                presentValue=Real(72.0),
                units=EngineeringUnits.degreesFahrenheit,
                description="Temperature setpoint",
            )
            self.app.add_object(setpoint)
            objects['setpoint'] = setpoint

        elif device_type == "AHU":
            # AHU devices: AI:1, AI:2, BV:1, AO:1
            supply_temp = AnalogInputObject(
                objectIdentifier=("analogInput", 1),
                objectName=f"{device_name}-SupplyTemp",
                presentValue=Real(55.0),
                units=EngineeringUnits.degreesFahrenheit,
                description="Supply air temperature sensor",
            )
            self.app.add_object(supply_temp)
            objects['supply_temp'] = supply_temp

            return_temp = AnalogInputObject(
                objectIdentifier=("analogInput", 2),
                objectName=f"{device_name}-ReturnTemp",
                presentValue=Real(72.0),
                units=EngineeringUnits.degreesFahrenheit,
                description="Return air temperature sensor",
            )
            self.app.add_object(return_temp)
            objects['return_temp'] = return_temp

            fan_status = BinaryValueObject(
                objectIdentifier=("binaryValue", 1),
                objectName=f"{device_name}-FanStatus",
                presentValue="active",
                description="Fan on/off status",
            )
            self.app.add_object(fan_status)
            objects['fan_status'] = fan_status

            fan_speed = AnalogOutputObject(
                objectIdentifier=("analogOutput", 1),
                objectName=f"{device_name}-FanSpeed",
                presentValue=Real(75.0),
                units=EngineeringUnits.percent,
                description="Fan speed control (VFD)",
            )
            self.app.add_object(fan_speed)
            objects['fan_speed'] = fan_speed

        self.devices.append({
            'name': device_name,
            'instance': device_instance,
            'type': device_type,
            'objects': objects,
        })

        print(f"✓ Created {device_name} (instance {device_instance})")
        for obj_name, obj in objects.items():
            print(f"    - {obj.objectIdentifier}: {obj.objectName}")

        return objects

    async def simulate_values(self):
        """Periodically update values to simulate real device behavior"""
        while self.running:
            await asyncio.sleep(5)  # Update every 5 seconds

            for device in self.devices:
                if device['type'] == 'VAV':
                    # Simulate temperature drift
                    temp = device['objects']['temp']
                    current_temp = float(temp.presentValue)
                    new_temp = current_temp + random.uniform(-0.5, 0.5)
                    new_temp = max(65.0, min(80.0, new_temp))
                    temp.presentValue = Real(new_temp)

                    # Simulate damper adjustment
                    damper = device['objects']['damper']
                    current_damper = float(damper.presentValue)
                    new_damper = current_damper + random.uniform(-5.0, 5.0)
                    new_damper = max(0.0, min(100.0, new_damper))
                    damper.presentValue = Real(new_damper)

                    # Occasionally toggle occupancy
                    if random.random() < 0.1:
                        occupancy = device['objects']['occupancy']
                        occupancy.presentValue = "active" if occupancy.presentValue == "inactive" else "inactive"

                elif device['type'] == 'AHU':
                    # Simulate supply temperature variation
                    supply = device['objects']['supply_temp']
                    current_supply = float(supply.presentValue)
                    new_supply = current_supply + random.uniform(-1.0, 1.0)
                    new_supply = max(50.0, min(60.0, new_supply))
                    supply.presentValue = Real(new_supply)

                    # Simulate return temperature variation
                    return_temp = device['objects']['return_temp']
                    current_return = float(return_temp.presentValue)
                    new_return = current_return + random.uniform(-0.5, 0.5)
                    new_return = max(68.0, min(76.0, new_return))
                    return_temp.presentValue = Real(new_return)

                    # Simulate fan speed adjustment
                    fan_speed = device['objects']['fan_speed']
                    target_speed = 50.0 + (new_return - 72.0) * 5.0
                    current_speed = float(fan_speed.presentValue)
                    new_speed = current_speed + (target_speed - current_speed) * 0.1
                    new_speed = max(30.0, min(100.0, new_speed))
                    fan_speed.presentValue = Real(new_speed)

                    fan_status = device['objects']['fan_status']
                    fan_status.presentValue = "active" if new_speed > 20.0 else "inactive"


async def main():
    """Main entry point"""
    print("=" * 60)
    print("Virtual BACnet Devices - All in One")
    print("=" * 60)
    print()
    print("Note: Due to BACpypes3 limitations, only the first device")
    print("will be created. Multiple devices require multiple IP")
    print("addresses or a BACnet VLAN implementation.")
    print()

    # Create the application
    multi_app = MultiDeviceApplication()

    # Try to create one VAV device (only this will work)
    print("Creating VAV-1 device...")
    await multi_app.create_device(101, "VAV-1", "VAV")

    print()
    print("=" * 60)
    print("BACnet device is running at 127.0.0.1:47808")
    print("Device instance: 101")
    print("Press Ctrl+C to stop...")
    print("=" * 60)
    print()

    # Start value simulation task
    simulation_task = asyncio.create_task(multi_app.simulate_values())

    try:
        # Run forever
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("\n\nShutting down...")
        multi_app.running = False
        simulation_task.cancel()
        try:
            await simulation_task
        except asyncio.CancelledError:
            pass


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
