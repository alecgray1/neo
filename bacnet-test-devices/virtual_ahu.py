#!/usr/bin/env python3
"""
virtual_ahu.py - Simulates 2 Air Handling Unit (AHU) BACnet devices

Each AHU has:
- AI:1 - Supply Air Temperature
- AI:2 - Return Air Temperature
- BV:1 - Fan Status
- AO:1 - Fan Speed Control
"""
import sys
import asyncio
import random
from bacpypes3.debugging import bacpypes_debugging, ModuleLogger
from bacpypes3.argparse import SimpleArgumentParser
from bacpypes3.app import Application
from bacpypes3.local.device import DeviceObject
from bacpypes3.local.analog import AnalogValueObject, AnalogInputObject, AnalogOutputObject
from bacpypes3.local.binary import BinaryValueObject
from bacpypes3.primitivedata import Real
from bacpypes3.basetypes import EngineeringUnits

# Module logger
_debug = 0
_log = ModuleLogger(globals())


@bacpypes_debugging
class VirtualAHUApplication:
    """Application managing multiple virtual AHU devices"""

    def __init__(self):
        """Initialize the AHU application"""
        self.devices = []
        self.running = True

    async def create_ahu_device(self, device_instance: int, device_name: str, address: str):
        """Create a single AHU device with all its objects"""

        # Create the device object
        device_object = DeviceObject(
            objectName=device_name,
            objectIdentifier=("device", device_instance),
            maxApduLengthAccepted=1024,
            segmentationSupported="segmentedBoth",
            vendorIdentifier=15,
            vendorName="Neo Virtual Device",
            modelName="Virtual AHU Unit",
        )

        # Create the application for this device
        app = Application(device_object, address)

        # Add Supply Air Temperature (AI:1)
        supply_temp = AnalogInputObject(
            objectIdentifier=("analogInput", 1),
            objectName=f"{device_name}-SupplyTemp",
            presentValue=Real(55.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Supply air temperature sensor",
        )
        app.add_object(supply_temp)

        # Add Return Air Temperature (AI:2)
        return_temp = AnalogInputObject(
            objectIdentifier=("analogInput", 2),
            objectName=f"{device_name}-ReturnTemp",
            presentValue=Real(72.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Return air temperature sensor",
        )
        app.add_object(return_temp)

        # Add Fan Status (BV:1)
        fan_status = BinaryValueObject(
            objectIdentifier=("binaryValue", 1),
            objectName=f"{device_name}-FanStatus",
            presentValue="active",
            description="Fan on/off status",
        )
        app.add_object(fan_status)

        # Add Fan Speed Control (AO:1)
        fan_speed = AnalogOutputObject(
            objectIdentifier=("analogOutput", 1),
            objectName=f"{device_name}-FanSpeed",
            presentValue=Real(75.0),
            units=EngineeringUnits.percent,
            description="Fan speed control (VFD)",
        )
        app.add_object(fan_speed)

        self.devices.append({
            'app': app,
            'name': device_name,
            'instance': device_instance,
            'objects': {
                'supply_temp': supply_temp,
                'return_temp': return_temp,
                'fan_status': fan_status,
                'fan_speed': fan_speed,
            }
        })

        print(f"✓ Created {device_name} (instance {device_instance}) at {address}")
        print(f"  - AI:1 Supply Air Temp: {supply_temp.presentValue}°F")
        print(f"  - AI:2 Return Air Temp: {return_temp.presentValue}°F")
        print(f"  - BV:1 Fan Status: {fan_status.presentValue}")
        print(f"  - AO:1 Fan Speed: {fan_speed.presentValue}%")

        return app

    async def simulate_values(self):
        """Periodically update values to simulate real device behavior"""
        while self.running:
            await asyncio.sleep(5)  # Update every 5 seconds

            for device in self.devices:
                # Simulate supply temperature variation
                supply = device['objects']['supply_temp']
                current_supply = float(supply.presentValue)
                new_supply = current_supply + random.uniform(-1.0, 1.0)
                new_supply = max(50.0, min(60.0, new_supply))  # Clamp to realistic range
                supply.presentValue = Real(new_supply)

                # Simulate return temperature variation
                return_temp = device['objects']['return_temp']
                current_return = float(return_temp.presentValue)
                new_return = current_return + random.uniform(-0.5, 0.5)
                new_return = max(68.0, min(76.0, new_return))  # Clamp to realistic range
                return_temp.presentValue = Real(new_return)

                # Simulate fan speed adjustment based on temperature
                fan_speed = device['objects']['fan_speed']
                # Higher return temp = higher fan speed
                target_speed = 50.0 + (new_return - 72.0) * 5.0
                current_speed = float(fan_speed.presentValue)
                new_speed = current_speed + (target_speed - current_speed) * 0.1
                new_speed = max(30.0, min(100.0, new_speed))
                fan_speed.presentValue = Real(new_speed)

                # Fan status follows speed (off if < 20%)
                fan_status = device['objects']['fan_status']
                fan_status.presentValue = "active" if new_speed > 20.0 else "inactive"

                if _debug:
                    print(f"📊 {device['name']}: Supply={new_supply:.1f}°F, Return={new_return:.1f}°F, "
                          f"Fan={fan_status.presentValue} @ {new_speed:.1f}%")


async def main():
    """Main entry point"""
    print("=" * 60)
    print("Virtual AHU BACnet Devices")
    print("=" * 60)
    print()

    # Create the application
    ahu_app = VirtualAHUApplication()

    # Create 2 AHU devices
    # Using different ports on localhost
    base_instance = 201
    for i in range(2):
        device_instance = base_instance + i
        device_name = f"AHU-{i+1}"
        # Each device gets its own port (after VAV devices)
        port = 47813 + i  # 47813, 47814
        address = f"127.0.0.1:{port}"

        await ahu_app.create_ahu_device(device_instance, device_name, address)

    print()
    print("=" * 60)
    print("All AHU devices are running!")
    print("Press Ctrl+C to stop...")
    print("=" * 60)
    print()

    # Start value simulation task
    simulation_task = asyncio.create_task(ahu_app.simulate_values())

    try:
        # Run forever
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("\n\nShutting down AHU devices...")
        ahu_app.running = False
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
